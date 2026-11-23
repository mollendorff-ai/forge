//! A folder the MCP server and the REST API must stay inside
//! (`forge mcp --root DIR`, `forge serve --root DIR`; roadmap T1-MCP-ROOT).
//!
//! Both servers read and write the files a request names, as the user who
//! runs them. An agent, or a prompt injected into one, can name any file
//! that user can reach. With a root, each path a request names is resolved
//! (`..` and symbolic links followed, as the system would) and refused unless
//! it is inside the root; so is each file a model includes, before it is read.
//! A relative path is relative to the root. Without a root nothing changes.
//!
//! The check runs before the command does. A process that can change links
//! inside the root while a request runs can get around it; the root is a
//! limit for what requests name, not a sandbox.

use std::path::{Path, PathBuf};

/// The folder requests must stay inside, resolved once at start.
#[derive(Debug, Clone)]
pub struct Root {
    dir: PathBuf,
}

impl Root {
    /// The root `dir`, which must be an existing folder.
    ///
    /// # Errors
    ///
    /// Returns an error if `dir` does not exist or is not a folder.
    pub fn new(dir: &Path) -> Result<Self, String> {
        let resolved = dir
            .canonicalize()
            .map_err(|e| format!("--root {}: {e}", dir.display()))?;
        if !resolved.is_dir() {
            return Err(format!("--root {}: not a folder", dir.display()));
        }
        Ok(Self { dir: resolved })
    }

    /// The folder, resolved.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// A file a request reads or writes: the resolved path, if it is inside
    /// the root. The file need not exist yet.
    ///
    /// # Errors
    ///
    /// Returns an error saying why, for a path outside the root.
    pub fn file(&self, path: &str) -> Result<PathBuf, String> {
        let path = Path::new(path);
        let full = if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.dir.join(path)
        };
        self.resolve(&full)
    }

    /// A model a request reads: [`Root::file`], and every file it includes,
    /// directly or through another include, inside the root too.
    ///
    /// # Errors
    ///
    /// Returns an error naming the model or the included file that is outside.
    pub fn model(&self, path: &str) -> Result<PathBuf, String> {
        let resolved = self.file(path)?;
        crate::parser::check_included_files(&resolved, &|p| self.resolve(p).map(|_| ()))
            .map_err(|e| format!("An included file: {e}"))?;
        Ok(resolved)
    }

    /// `path` (absolute) with `..` and links resolved. For a path that does
    /// not exist yet, the deepest part that exists is resolved and the
    /// missing names are added to it.
    fn resolve(&self, path: &Path) -> Result<PathBuf, String> {
        let mut existing = path.to_path_buf();
        let mut missing = Vec::new();
        let resolved = loop {
            if let Ok(resolved) = existing.canonicalize() {
                break resolved;
            }
            if existing.symlink_metadata().is_ok() {
                // A link to a file that does not exist: writing it would
                // create the file the link points to, wherever that is.
                return Err(format!(
                    "'{}' is a link to a missing file, which may be outside {}",
                    path.display(),
                    self.describe()
                ));
            }
            // A name ending in `..` under a missing folder cannot be resolved
            let name = existing.file_name().ok_or_else(|| self.outside(path))?;
            missing.push(name.to_owned());
            // A path with a file name has a parent, so this always pops
            existing.pop();
        };
        let resolved = missing
            .into_iter()
            .rev()
            .fold(resolved, |p, name| p.join(name));
        if resolved.starts_with(&self.dir) {
            Ok(resolved)
        } else {
            Err(self.outside(path))
        }
    }

    fn outside(&self, path: &Path) -> String {
        format!("'{}' is outside {}", path.display(), self.describe())
    }

    fn describe(&self) -> String {
        format!(
            "{}, the folder this server may use (--root); a relative path is relative to it",
            self.dir.display()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn a_name_that_only_looks_like_the_root_is_outside() {
        // `/x/models2` starts with the text `/x/models` but is not inside it
        let base = tempfile::TempDir::new().unwrap();
        fs::create_dir(base.path().join("models")).unwrap();
        fs::create_dir(base.path().join("models2")).unwrap();
        let root = Root::new(&base.path().join("models")).unwrap();
        let other = base.path().join("models2").join("m.yaml");
        assert!(root.file(other.to_str().unwrap()).is_err());
    }

    #[test]
    fn a_file_that_does_not_exist_yet_is_placed_under_its_resolved_folder() {
        let base = tempfile::TempDir::new().unwrap();
        let root = Root::new(base.path()).unwrap();
        let path = root.file("new/deeper/out.xlsx").unwrap();
        assert_eq!(path, root.dir().join("new/deeper/out.xlsx"));
        assert!(root.file("new/../../out.xlsx").is_err());
    }

    #[test]
    fn the_root_itself_is_inside() {
        let base = tempfile::TempDir::new().unwrap();
        let root = Root::new(base.path()).unwrap();
        assert_eq!(root.file(".").unwrap(), root.dir());
    }
}
