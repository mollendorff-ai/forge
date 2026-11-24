//! The Studio guide's pictures and the tests it names.
//!
//! ADR-078, roadmap A8: `docs/guide/studio.md` shows Forge Studio, so every
//! picture on it is made from the running page by `studio/screenshots.mjs`
//! and none is anything else. Its commands and links are the `guide`
//! check's, like the lessons'; what a screen shows cannot be run here, so the
//! page names the tests in `studio/tests/` that hold what it says, and each
//! of them has to be there.

use std::path::Path;

/// The pictures of a page, each as its alternative text and its path:
/// `![text](img/studio-grid.png)`.
#[must_use]
pub fn pictures_in(doc: &str) -> Vec<(String, String)> {
    doc.match_indices("![")
        .filter_map(|(at, _)| {
            let rest = &doc[at + 2..];
            let (alt, after) = rest.split_once("](")?;
            let (path, _) = after.split_once(')')?;
            Some((alt.to_string(), path.to_string()))
        })
        .collect()
}

/// The file names the script writes for the guide: `join(guide, "x.png")`.
#[must_use]
pub fn pictures_made(script: &str) -> Vec<String> {
    script
        .match_indices("join(guide, \"")
        .filter_map(|(at, marker)| {
            let rest = &script[at + marker.len()..];
            Some(rest.split_once('"')?.0.to_string())
        })
        .collect()
}

/// What is wrong with the pictures `shown` on a page.
///
/// Given the file names the script `made` and which paths `exists`: a picture
/// without alternative text, outside `img/`, with no file, not made by the
/// script, or made by the script and not shown.
#[must_use]
pub fn picture_faults(
    shown: &[(String, String)],
    made: &[String],
    exists: &dyn Fn(&str) -> bool,
) -> Vec<String> {
    let mut faults = Vec::new();
    for (alt, path) in shown {
        let name = path.strip_prefix("img/");
        if alt.trim().is_empty() {
            faults.push(format!("{path} has no alternative text"));
        }
        if name.is_none() {
            faults.push(format!("{path} is not in img/"));
        }
        if !exists(path) {
            faults.push(format!("{path} is not a file"));
        }
        if !name.is_some_and(|name| made.iter().any(|m| m == name)) {
            faults.push(format!("studio/screenshots.mjs does not make {path}"));
        }
    }
    for name in made {
        if !shown.iter().any(|(_, path)| path == &format!("img/{name}")) {
            faults.push(format!("the page does not show {name}"));
        }
    }
    faults
}

/// The `.mjs` files a text names, by file name, each once.
#[must_use]
pub fn scripts_named(doc: &str) -> Vec<String> {
    let mut found: Vec<String> = Vec::new();
    for word in doc.split(|c: char| !(c.is_ascii_alphanumeric() || "_.-/".contains(c))) {
        let path = Path::new(word.trim_end_matches('.'));
        let name = path.file_name().and_then(|n| n.to_str()).filter(|_| {
            path.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("mjs"))
        });
        if let Some(name) = name {
            if !found.iter().any(|f| f == name) {
                found.push(name.to_string());
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read, root};

    fn shown(alt: &str, path: &str) -> Vec<(String, String)> {
        vec![(alt.to_string(), path.to_string())]
    }

    #[test]
    fn pictures_are_read_with_their_text_and_path() {
        let doc = "A ![the grid](img/a.png) and ![](img/b.png), not ![broken](img/c.png";
        assert_eq!(
            pictures_in(doc),
            [
                ("the grid".to_string(), "img/a.png".to_string()),
                (String::new(), "img/b.png".to_string()),
            ]
        );
        assert_eq!(pictures_in("![no path"), Vec::<(String, String)>::new());
    }

    #[test]
    fn the_script_names_its_pictures() {
        let script = "await page.screenshot({ path: join(guide, \"a.png\") });\n\
                      await page.screenshot({ path: join(out, \"b.png\") });\n\
                      x(join(guide, \"c.png\"));\njoin(guide, \"open";
        assert_eq!(pictures_made(script), ["a.png", "c.png"]);
    }

    #[test]
    fn a_picture_must_have_text_a_file_and_a_script_that_makes_it() {
        let made = vec!["a.png".to_string()];
        let there = |_: &str| true;
        assert_eq!(
            picture_faults(&shown("text", "img/a.png"), &made, &there),
            Vec::<String>::new()
        );
        assert_eq!(
            picture_faults(&shown(" ", "img/a.png"), &made, &there),
            ["img/a.png has no alternative text"]
        );
        assert_eq!(
            picture_faults(&shown("t", "img/a.png"), &made, &|_: &str| false),
            ["img/a.png is not a file"]
        );
        assert_eq!(
            picture_faults(&shown("t", "other/a.png"), &made, &there),
            [
                "other/a.png is not in img/",
                "studio/screenshots.mjs does not make other/a.png",
                "the page does not show a.png",
            ]
        );
        assert_eq!(
            picture_faults(&shown("t", "img/b.png"), &made, &there),
            [
                "studio/screenshots.mjs does not make img/b.png",
                "the page does not show a.png",
            ]
        );
        assert_eq!(
            picture_faults(&[], &made, &there),
            ["the page does not show a.png"]
        );
    }

    #[test]
    fn scripts_are_named_by_file_once() {
        let doc = "[s](../../studio/screenshots.mjs) and `screen.mjs`, `studio/tests/screen.mjs`, grid.mjs.";
        assert_eq!(
            scripts_named(doc),
            ["screenshots.mjs", "screen.mjs", "grid.mjs"]
        );
        assert_eq!(scripts_named("no scripts, .mjs"), Vec::<String>::new());
    }

    #[test]
    fn the_studio_guide_shows_only_pictures_the_script_makes_and_names_tests_that_exist() {
        let doc = read("docs/guide/studio.md");
        let page = root().join("docs/guide");
        let faults = picture_faults(
            &pictures_in(&doc),
            &pictures_made(&read("studio/screenshots.mjs")),
            &|path| page.join(path).is_file(),
        );
        assert_eq!(faults, Vec::<String>::new());
        let named = scripts_named(&doc);
        assert!(named.len() > 5, "{named:?}");
        for script in named {
            let there = ["studio", "studio/tests"]
                .iter()
                .any(|dir| root().join(dir).join(&script).is_file());
            assert!(there, "the page names {script}, which is not in studio/");
        }
    }

    #[test]
    fn the_numbers_the_guide_quotes_are_the_ones_the_first_lesson_runs() {
        let studio = read("docs/guide/studio.md");
        let lesson = read("docs/guide/01-first-ten-minutes.md");
        for (shown, printed) in [("24,334.44", "24334.44"), ("786.32", "786.32")] {
            assert!(studio.contains(shown), "{shown}");
            assert!(lesson.contains(printed), "{printed}");
        }
    }
}
