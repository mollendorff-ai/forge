//! `forge mcp --root DIR` (T1-MCP-ROOT): a path outside DIR is refused, for
//! reading and for writing, through `..`, a symbolic link or an included file.
//! Without a root nothing changes.

use super::*;
use crate::path_root::Root;
use rmcp::handler::server::wrapper::Parameters;
use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const MODEL: &str = "_forge_version: \"5.1.0\"\na:\n  x: 2\n  y: {formula: \"=a.x * 3\"}\n";

/// `models/` is the root; `secret/` is beside it, outside.
struct Folders {
    _base: TempDir,
    root: PathBuf,
    secret: PathBuf,
}

fn folders() -> Folders {
    let base = TempDir::new().unwrap();
    let root = base.path().join("models");
    let secret = base.path().join("secret");
    fs::create_dir(&root).unwrap();
    fs::create_dir(&secret).unwrap();
    fs::write(root.join("m.yaml"), MODEL).unwrap();
    fs::write(secret.join("s.yaml"), MODEL).unwrap();
    // A link to a file outside, and a link to the folder outside
    symlink(secret.join("s.yaml"), root.join("link.yaml")).unwrap();
    symlink(&secret, root.join("out")).unwrap();
    // A model inside that includes a file outside
    fs::write(
        root.join("inc.yaml"),
        format!("{MODEL}_includes:\n  - file: ../secret/s.yaml\n    as: s\n"),
    )
    .unwrap();
    Folders {
        root,
        secret,
        _base: base,
    }
}

fn server(f: &Folders) -> ForgeMcpServer {
    ForgeMcpServer::new().with_root(Root::new(&f.root).unwrap())
}

fn s(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}

fn validate(server: &ForgeMcpServer, path: &str) -> Result<String, String> {
    server.validate(Parameters(ValidateRequest {
        file_path: Some(path.to_string()),
        content: None,
        includes: None,
        verbose: false,
    }))
}

fn export(server: &ForgeMcpServer, yaml: &str, excel: &str) -> Result<String, String> {
    server.export(Parameters(ExportRequest {
        yaml_path: Some(yaml.to_string()),
        content: None,
        includes: None,
        excel_path: Some(excel.to_string()),
    }))
}

fn refused(result: Result<String, String>) -> String {
    let err = result.expect_err("a path outside the root must be refused");
    assert!(err.contains("outside"), "says why: {err}");
    err
}

#[test]
fn a_model_inside_the_root_is_read_by_full_or_relative_path() {
    let f = folders();
    let server = server(&f);
    validate(&server, &s(&f.root.join("m.yaml"))).unwrap();
    validate(&server, "m.yaml").unwrap();
}

#[test]
fn a_model_outside_the_root_is_refused() {
    let f = folders();
    let err = refused(validate(&server(&f), &s(&f.secret.join("s.yaml"))));
    assert!(err.contains("--root"), "names the setting: {err}");
}

#[test]
fn dot_dot_cannot_leave_the_root() {
    let f = folders();
    let server = server(&f);
    refused(validate(&server, "../secret/s.yaml"));
    refused(validate(&server, &s(&f.root.join("../secret/s.yaml"))));
    // Through a folder that does not exist, too
    refused(validate(&server, "missing/../../secret/s.yaml"));
}

#[test]
fn a_symbolic_link_cannot_leave_the_root() {
    let f = folders();
    let server = server(&f);
    refused(validate(&server, "link.yaml"));
    refused(validate(&server, "out/s.yaml"));
}

#[test]
fn an_included_file_outside_the_root_is_refused() {
    let f = folders();
    let err = refused(validate(&server(&f), "inc.yaml"));
    assert!(err.contains("s.yaml"), "names the included file: {err}");
}

#[test]
fn export_cannot_write_outside_the_root() {
    let f = folders();
    let server = server(&f);
    refused(export(&server, "m.yaml", &s(&f.secret.join("x.xlsx"))));
    refused(export(&server, "m.yaml", "../secret/x.xlsx"));
    refused(export(&server, "m.yaml", "out/x.xlsx"));
    assert!(!f.secret.join("x.xlsx").exists(), "nothing written outside");
    export(&server, "m.yaml", "m.xlsx").unwrap();
    assert!(f.root.join("m.xlsx").exists(), "relative to the root");
}

#[test]
fn a_link_where_the_file_will_be_written_cannot_leave_the_root() {
    let f = folders();
    // A link that points outside to a file that does not exist yet
    symlink(f.secret.join("new.xlsx"), f.root.join("dangling.xlsx")).unwrap();
    refused(export(&server(&f), "m.yaml", "dangling.xlsx"));
    assert!(!f.secret.join("new.xlsx").exists());
}

#[test]
fn import_cannot_read_or_write_outside_the_root() {
    let f = folders();
    let server = server(&f);
    export(&server, "m.yaml", "m.xlsx").unwrap();
    fs::copy(f.root.join("m.xlsx"), f.secret.join("m.xlsx")).unwrap();
    let import = |excel: &str, yaml: &str| {
        server.import(Parameters(ImportRequest {
            excel_path: excel.to_string(),
            yaml_path: yaml.to_string(),
        }))
    };
    refused(import(&s(&f.secret.join("m.xlsx")), "back.yaml"));
    refused(import("m.xlsx", &s(&f.secret.join("back.yaml"))));
    assert!(
        !f.secret.join("back.yaml").exists(),
        "nothing written outside"
    );
    import("m.xlsx", "back.yaml").unwrap();
    assert!(f.root.join("back.yaml").exists());
}

#[test]
fn both_files_of_a_variance_are_checked() {
    let f = folders();
    let result = server(&f).variance(Parameters(VarianceRequest {
        budget_path: Some("m.yaml".into()),
        budget_content: None,
        actual_path: Some(s(&f.secret.join("s.yaml"))),
        actual_content: None,
        includes: None,
        threshold: None,
    }));
    refused(result);
}

#[test]
fn inline_content_still_works_with_a_root() {
    let f = folders();
    server(&f)
        .validate(Parameters(ValidateRequest {
            file_path: None,
            content: Some(MODEL.into()),
            includes: None,
            verbose: false,
        }))
        .unwrap();
}

#[test]
fn without_a_root_nothing_changes() {
    let f = folders();
    validate(&ForgeMcpServer::new(), &s(&f.secret.join("s.yaml"))).unwrap();
}

#[test]
fn the_server_tells_the_agent_where_files_must_be() {
    let f = folders();
    let info = server(&f).get_info();
    let text = info.instructions.unwrap_or_default();
    let root = f.root.canonicalize().unwrap();
    assert!(text.contains(&s(&root)), "{text}");
}

#[test]
fn a_root_that_is_not_a_folder_is_refused_at_start() {
    let f = folders();
    assert!(Root::new(&f.root.join("m.yaml")).is_err());
    assert!(Root::new(&f.root.join("missing")).is_err());
}

#[test]
fn an_include_of_an_include_outside_the_root_is_refused() {
    let f = folders();
    fs::write(
        f.root.join("outer.yaml"),
        format!("{MODEL}_includes:\n  - file: inc.yaml\n    as: inner\n"),
    )
    .unwrap();
    refused(validate(&server(&f), "outer.yaml"));
}

#[test]
fn a_document_of_a_multi_document_model_cannot_include_outside() {
    let f = folders();
    let second = "_forge_version: \"5.1.0\"\n_includes:\n  - file: ../secret/s.yaml\n    as: s\nb:\n  z: 1\n";
    fs::write(
        f.root.join("multi.yaml"),
        format!("---\n{MODEL}---\n{second}"),
    )
    .unwrap();
    refused(validate(&server(&f), "multi.yaml"));
}
