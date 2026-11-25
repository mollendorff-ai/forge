//! Lanes, their roadmaps and the changelog's fragments hold together.
//!
//! Forge is worked on in lanes, several sessions at once (`AGENTS.md`,
//! roadmap E-LANES): each lane in `.asimov/project.yaml` has its own
//! roadmap in `.asimov/roadmap/`, and a change's changelog entry is a
//! fragment in `changelog.d/` that scriv collects at a release.
//!
//! These checks keep that true: every lane has its roadmap and every
//! roadmap a lane, no item id is in two places, `AGENTS.md` names each
//! lane, each fragment has only the headings scriv sorts, and nothing is
//! written under `[Unreleased]` by hand.

use serde_json::Value;

/// The headings a changelog fragment may use, the categories of
/// `changelog.d/scriv.ini`.
pub const CATEGORIES: [&str; 4] = ["Added", "Changed", "Fixed", "Removed"];

/// The marker under `## [Unreleased]` where scriv writes a release.
pub const MARKER: &str = "<!-- scriv-insert-here -->";

/// The ids of the lanes in a project file, in order.
#[must_use]
pub fn lane_ids(project: &Value) -> Vec<String> {
    project["lanes"]
        .as_array()
        .map(|lanes| {
            lanes
                .iter()
                .filter_map(|lane| lane["id"].as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// Every item id in a roadmap: its `current` item's, and those of the items
/// in each of its lists (`backlog`, `done`, a tier), section by section in
/// name order.
#[must_use]
pub fn item_ids(roadmap: &Value) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    let Some(sections) = roadmap.as_object() else {
        return ids;
    };
    for section in sections.values() {
        if let Some(id) = section["id"].as_str() {
            ids.push(id.to_string());
        }
        for item in section.as_array().into_iter().flatten() {
            ids.extend(item["id"].as_str().map(str::to_string));
        }
    }
    ids
}

/// The ids that appear more than once, each once, in the order first seen.
#[must_use]
pub fn repeated(ids: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for (at, id) in ids.iter().enumerate() {
        if ids[..at].contains(id) && !out.contains(id) {
            out.push(id.clone());
        }
    }
    out
}

/// What is wrong with a changelog fragment: a heading that is not one of
/// [`CATEGORIES`] at level 3, text before the first heading, a heading with
/// no entry under it, or no heading at all.
#[must_use]
pub fn fragment_problems(text: &str) -> Vec<String> {
    let mut problems: Vec<String> = Vec::new();
    let mut heading: Option<&str> = None;
    let mut entries = 0;
    let close = |heading: Option<&str>, entries: usize, problems: &mut Vec<String>| {
        if let Some(h) = heading.filter(|_| entries == 0) {
            problems.push(format!("{h} has no entry under it"));
        }
    };
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        if line.starts_with('#') {
            close(heading, entries, &mut problems);
            let category = line.strip_prefix("### ").unwrap_or_default();
            if !CATEGORIES.contains(&category) {
                problems.push(format!("{line} is not ### and one of {CATEGORIES:?}"));
            }
            heading = Some(line);
            entries = 0;
        } else if heading.is_none() {
            problems.push(format!("text before the first heading: {line}"));
        } else if line.starts_with("- ") {
            entries += 1;
        }
    }
    close(heading, entries, &mut problems);
    if heading.is_none() {
        problems.push("no heading".to_string());
    }
    problems
}

/// The lines under `## [Unreleased]` in a changelog, up to the next release,
/// that are neither blank nor a comment: what was written there by hand.
#[must_use]
pub fn written_under_unreleased(changelog: &str) -> Vec<String> {
    changelog
        .split_once("## [Unreleased]")
        .map(|(_, after)| after.split("\n## [").next().unwrap_or_default())
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !(l.is_empty() || l.starts_with("<!--") && l.ends_with("-->")))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read, root};
    use serde_json::json;

    fn yaml(path: &str) -> Value {
        let parsed = yaml_serde::from_str::<Value>(&read(path));
        assert!(parsed.is_ok(), "{path} is not YAML");
        parsed.unwrap_or_default()
    }

    fn names(dir: &str) -> Vec<String> {
        let mut out: Vec<String> = std::fs::read_dir(root().join(dir))
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        out.sort();
        out
    }

    #[test]
    fn lane_ids_are_read_in_order_and_a_lane_without_one_is_skipped() {
        let project = json!({"lanes": [{"id": "engine"}, {"title": "x"}, {"id": "manual"}]});
        assert_eq!(lane_ids(&project), ["engine", "manual"]);
        assert_eq!(lane_ids(&json!({"identity": {}})), Vec::<String>::new());
    }

    #[test]
    fn item_ids_come_from_current_and_every_list() {
        let roadmap = json!({
            "state": {"version": "x"},
            "current": {"id": "E-1"},
            "backlog": [{"id": "E-2"}, {"title": "no id"}],
            "done": [{"id": "E-0"}],
            "tier3": []
        });
        // the sections in name order, as serde_json keeps a map
        assert_eq!(item_ids(&roadmap), ["E-2", "E-1", "E-0"]);
        assert_eq!(item_ids(&json!("not a map")), Vec::<String>::new());
    }

    #[test]
    fn a_repeated_id_is_named_once() {
        let ids: Vec<String> = ["a", "b", "a", "c", "a", "b"].map(String::from).to_vec();
        assert_eq!(repeated(&ids), ["a", "b"]);
        assert_eq!(repeated(&ids[..2]), Vec::<String>::new());
    }

    #[test]
    fn a_fragment_with_known_headings_and_entries_passes() {
        let text = "### Fixed\n\n- **One.** It was wrong.\n  Still the same entry.\n\n### Added\n\n- **Two.**\n";
        assert_eq!(fragment_problems(text), Vec::<String>::new());
    }

    #[test]
    fn a_fragment_with_another_heading_text_first_or_no_entry_is_named() {
        assert_eq!(
            fragment_problems("Intro\n### Fixes\n- x\n## Added\n- y\n### Removed\n"),
            [
                "text before the first heading: Intro",
                "### Fixes is not ### and one of [\"Added\", \"Changed\", \"Fixed\", \"Removed\"]",
                "## Added is not ### and one of [\"Added\", \"Changed\", \"Fixed\", \"Removed\"]",
                "### Removed has no entry under it",
            ]
        );
        assert_eq!(
            fragment_problems("- an entry\n"),
            ["text before the first heading: - an entry", "no heading"]
        );
        assert_eq!(fragment_problems(""), ["no heading"]);
    }

    #[test]
    fn only_comments_under_unreleased_count_as_empty() {
        let empty = format!(
            "# Changelog\n\n## [Unreleased]\n\n<!-- note -->\n{MARKER}\n\n## [1.0] - d\n\n- old\n"
        );
        assert_eq!(written_under_unreleased(&empty), Vec::<String>::new());
        let written = "## [Unreleased]\n\n### Added\n\n- new\n\n## [1.0] - d\n- old\n";
        assert_eq!(written_under_unreleased(written), ["### Added", "- new"]);
        assert_eq!(
            written_under_unreleased("## [Unreleased]\n- only\n"),
            ["- only"]
        );
        assert_eq!(
            written_under_unreleased("# no section\n"),
            Vec::<String>::new()
        );
    }

    #[test]
    fn every_lane_has_a_roadmap_and_every_roadmap_a_lane() {
        let lanes = lane_ids(&yaml(".asimov/project.yaml"));
        assert_eq!(lanes, ["engine", "manual", "features", "architecture"]);
        let mut expected: Vec<String> = lanes.iter().map(|id| format!("{id}.yaml")).collect();
        expected.sort();
        assert_eq!(names(".asimov/roadmap"), expected);
        for id in &lanes {
            let roadmap = yaml(&format!(".asimov/roadmap/{id}.yaml"));
            assert!(
                roadmap["current"].is_object(),
                "{id}: current is not a mapping"
            );
            assert!(roadmap["backlog"].is_array(), "{id}: backlog is not a list");
            assert!(roadmap["done"].is_array(), "{id}: done is not a list");
        }
    }

    #[test]
    fn each_lane_owns_paths_and_only_the_engine_releases() {
        let project = yaml(".asimov/project.yaml");
        let lanes = project["lanes"].as_array().cloned().unwrap_or_default();
        for lane in &lanes {
            let owns = lane["owns"].as_array().map_or(0, Vec::len);
            assert!(owns > 0, "{} owns nothing", lane["id"]);
        }
        let releasing: Vec<&Value> = lanes
            .iter()
            .filter(|l| l["releases"] == json!(true))
            .collect();
        assert_eq!(releasing.len(), 1);
        assert_eq!(releasing[0]["id"], "engine");
        assert_eq!(project["changelog"]["fragments"], "changelog.d/");
    }

    #[test]
    fn no_item_id_is_in_two_places() {
        let mut ids = item_ids(&yaml(".asimov/roadmap.yaml"));
        for id in lane_ids(&yaml(".asimov/project.yaml")) {
            ids.extend(item_ids(&yaml(&format!(".asimov/roadmap/{id}.yaml"))));
        }
        assert!(ids.len() > 50, "only {} items read", ids.len());
        assert_eq!(repeated(&ids), Vec::<String>::new());
    }

    #[test]
    fn agents_md_names_each_lane_and_its_roadmap() {
        let agents = read("AGENTS.md");
        for id in lane_ids(&yaml(".asimov/project.yaml")) {
            let row = format!("| `{id}` |");
            assert!(agents.contains(&row), "AGENTS.md has no row {row}");
            let roadmap = format!("`.asimov/roadmap/{id}.yaml`");
            assert!(
                agents.contains(&roadmap),
                "AGENTS.md does not name {roadmap}"
            );
        }
    }

    #[test]
    fn every_changelog_fragment_has_only_the_headings_scriv_sorts() {
        let ini = read("changelog.d/scriv.ini");
        assert!(ini.contains(&format!("categories = {}", CATEGORIES.join(", "))));
        for name in names("changelog.d") {
            if name == "README.md" || name == "scriv.ini" {
                continue;
            }
            assert!(
                std::path::Path::new(&name)
                    .extension()
                    .is_some_and(|x| x == "md"),
                "changelog.d/{name} is not a Markdown fragment"
            );
            let text = read(&format!("changelog.d/{name}"));
            assert_eq!(
                fragment_problems(&text),
                Vec::<String>::new(),
                "changelog.d/{name}"
            );
        }
    }

    #[test]
    fn nothing_is_written_under_unreleased_by_hand() {
        let changelog = read("CHANGELOG.md");
        assert!(changelog.contains(&format!("## [Unreleased]\n\n<!-- New entries go in changelog.d/, not here (changelog.d/README.md). -->\n{MARKER}\n")));
        assert_eq!(written_under_unreleased(&changelog), Vec::<String>::new());
    }
}
