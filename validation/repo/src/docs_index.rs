//! The index of the documentation, `docs/README.md`.
//!
//! ADR-078, roadmap A10: the index names each reader's first page and then
//! every page of `docs/` by what it holds, so a page nobody links is one a
//! reader cannot find. The check fails when a page is missing from the
//! index, when a link of it goes nowhere, and when the main README's
//! documentation table does not lead to it.

use crate::architecture_docs::local_links;

/// The pages of `pages` (paths from the top, `docs/guide/studio.md`) that
/// `index`, a page in the folder `base` (`docs/`), does not link.
#[must_use]
pub fn unlinked(index: &str, base: &str, pages: &[String]) -> Vec<String> {
    let links = local_links(index);
    pages
        .iter()
        .filter(|page| {
            let target = page.strip_prefix(base).unwrap_or(page);
            !links
                .iter()
                .any(|link| link.split('#').next() == Some(target))
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::guide::{check_links, pages_in};
    use crate::{read, root};

    fn names(pages: &[&str]) -> Vec<String> {
        pages.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_page_is_linked_when_the_index_links_it_with_or_without_an_anchor() {
        let index = "[a](guide/a.md), [b](b.md#part), `[c](c.md)`, [d](https://x/d.md)";
        let pages = names(&["docs/guide/a.md", "docs/b.md", "docs/c.md", "docs/d.md"]);
        assert_eq!(unlinked(index, "docs/", &pages), ["docs/c.md", "docs/d.md"]);
        assert_eq!(
            unlinked(
                "[x](../CONTRIBUTING.md)",
                "docs/",
                &names(&["CONTRIBUTING.md"])
            ),
            ["CONTRIBUTING.md"]
        );
    }

    #[test]
    fn every_page_of_docs_is_in_the_index_and_its_links_go_somewhere() {
        let index = read("docs/README.md");
        let mut pages = Vec::new();
        for folder in ["docs", "docs/guide", "docs/cli"] {
            pages.extend(pages_in(&root().join(folder), folder));
        }
        pages.push("docs/posts/README.md".to_string());
        pages.push("docs/architecture/README.md".to_string());
        pages.retain(|page| page != "docs/README.md");
        assert!(pages.len() > 20, "{pages:?}");
        assert_eq!(unlinked(&index, "docs/", &pages), Vec::<String>::new());
        // the posts are the posts index's to link
        let mut posts = pages_in(&root().join("docs/posts"), "docs/posts");
        posts.retain(|page| page != "docs/posts/README.md");
        assert_eq!(posts.len(), 8, "{posts:?}");
        let posts_index = read("docs/posts/README.md");
        assert_eq!(
            unlinked(&posts_index, "docs/posts/", &posts),
            Vec::<String>::new()
        );
        let linked = check_links(&root().join("docs/README.md"), &index).unwrap();
        assert!(linked > 30, "{linked}");
    }

    #[test]
    fn the_readers_start_pages_are_the_ones_adr_078_names() {
        let index = read("docs/README.md");
        for start in [
            "guide/README.md",
            "guide/ai.md",
            "guide/pipeline.md",
            "EMBEDDING.md",
            "../CONTRIBUTING.md",
        ] {
            assert!(index.contains(&format!("]({start})")), "{start}");
        }
    }

    #[test]
    fn the_main_readme_leads_to_the_index() {
        let readme = read("README.md");
        let table = readme.split("## Documentation").nth(1).unwrap();
        assert!(table.contains("](docs/README.md)"));
    }
}
