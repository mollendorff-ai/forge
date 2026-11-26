//! The files that say what Forge is to a tool, not a reader: `CITATION.cff`
//! (GitHub's "Cite this repository") and `.well-known/security.txt`, each
//! checked against `Cargo.toml` and `SECURITY.md` so they cannot drift.

#[cfg(test)]
mod tests {
    use crate::read;

    /// The value of a top-level `key: "value"` (or TOML `key = "value"`) line.
    fn field(text: &str, key: &str) -> String {
        text.lines()
            .find_map(|l| {
                let rest = l.strip_prefix(key)?;
                rest.strip_prefix(": ").or_else(|| rest.strip_prefix(" = "))
            })
            .unwrap_or_default()
            .trim_matches('"')
            .to_string()
    }

    #[test]
    fn citation_has_the_crates_version_licence_and_addresses() {
        let (cargo, cff) = (read("Cargo.toml"), read("CITATION.cff"));
        let version = field(&cargo, "version");
        assert!(version.starts_with("10."), "Cargo.toml version {version}");
        assert_eq!(field(&cff, "version"), version);
        assert_eq!(field(&cff, "cff-version"), "1.2.0");
        assert_eq!(field(&cff, "repository-code"), field(&cargo, "repository"));
        assert_eq!(field(&cff, "url"), field(&cargo, "homepage"));
        assert_eq!(
            field(&cff, "abstract"),
            field(&cargo, "description").replace(" Release candidate.", "")
        );
        assert_eq!(field(&cargo, "license"), "MIT OR Apache-2.0");
        assert!(cff.contains("  - MIT\n  - Apache-2.0\n"));
    }

    #[test]
    fn security_txt_names_the_address_and_policy_of_security_md() {
        let (txt, policy) = (read(".well-known/security.txt"), read("SECURITY.md"));
        let contact = field(&txt, "Contact");
        assert!(contact.starts_with("mailto:"), "{contact}");
        assert!(policy.contains(&format!("**{}**", &contact["mailto:".len()..])));
        assert!(field(&txt, "Policy").ends_with("/SECURITY.md"));
        assert!(field(&txt, "Expires")
            .parse::<String>()
            .is_ok_and(|e| e.len() == 24));
    }
}
