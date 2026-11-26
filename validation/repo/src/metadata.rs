//! The files that tell a tool what Forge is, checked against the crate.
//!
//! `CITATION.cff` (GitHub's "Cite this repository"), `.well-known/security.txt`
//! and `server.json` (the MCP Registry's entry) are each held against
//! `Cargo.toml`, `SECURITY.md` and the binary, so they cannot drift.

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

    #[test]
    fn server_json_names_this_crate_version_and_command() {
        let entry: serde_json::Value = serde_json::from_str(&read("server.json")).unwrap();
        let cargo = read("Cargo.toml");
        let (version, name) = (field(&cargo, "version"), field(&cargo, "name"));
        assert_eq!(entry["version"], version.as_str());
        let package = &entry["packages"][0];
        assert_eq!(package["registryType"], "cargo");
        assert_eq!(package["registryBaseUrl"], "https://crates.io");
        assert_eq!(package["identifier"], name.as_str());
        assert_eq!(package["version"], version.as_str());
        assert_eq!(package["transport"]["type"], "stdio");
        assert_eq!(
            entry["repository"]["url"],
            field(&cargo, "repository").as_str()
        );
        assert_eq!(entry["websiteUrl"], field(&cargo, "homepage").as_str());
        assert!(entry["name"]
            .as_str()
            .is_some_and(|n| n.starts_with("io.github.mollendorff-ai/")));
        let length = entry["description"]
            .as_str()
            .map_or(0, |d| d.chars().count());
        assert!(
            (1..=100).contains(&length),
            "the registry takes 1 to 100 characters, not {length}"
        );
    }

    /// What the entry says to run is a command the binary has, and the
    /// binary speaks MCP as the version the entry names.
    #[test]
    fn server_json_command_is_what_the_binary_serves() {
        let entry: serde_json::Value = serde_json::from_str(&read("server.json")).unwrap();
        let argument = entry["packages"][0]["packageArguments"][0]["value"]
            .as_str()
            .unwrap_or_default();
        assert_eq!(argument, "mcp");
        let help = crate::forge(&[argument, "--help"]);
        assert_eq!(help.code, Some(0), "forge {argument} --help: {}", help.err);
        assert!(help.out.contains("MCP"));
        let version = crate::forge(&["--version"]);
        assert!(version
            .out
            .contains(entry["version"].as_str().unwrap_or_default()));
    }
}
