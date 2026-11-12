//! Spreadsheet engines that recalculate Forge's exported workbooks.
//!
//! Gnumeric (ssconvert) recalculates every case. `LibreOffice` recalculates the
//! cases whose functions Gnumeric 1.12 cannot read from an xlsx file (`LET`,
//! `UNIQUE`); each such case says why in its `libreoffice` key.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Gnumeric spreadsheet engine for formula recalculation.
pub struct GnumericEngine {
    /// Path to the ssconvert binary.
    path: PathBuf,
    /// Version string from ssconvert.
    version: String,
}

impl GnumericEngine {
    /// Engine name constant.
    pub const NAME: &'static str = "Gnumeric (ssconvert)";

    /// Detects Gnumeric (ssconvert) installation.
    ///
    /// Returns `Some(engine)` if ssconvert is found and working,
    /// `None` otherwise.
    #[must_use]
    pub fn detect() -> Option<Self> {
        let output = Command::new("ssconvert").arg("--version").output().ok()?;

        if output.status.success() {
            let version = String::from_utf8_lossy(&output.stderr).trim().to_string();
            Some(Self {
                path: PathBuf::from("ssconvert"),
                version,
            })
        } else {
            None
        }
    }

    /// Returns the engine version string.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Returns the engine name.
    #[must_use]
    pub const fn name() -> &'static str {
        Self::NAME
    }

    /// Converts XLSX to CSV with formula recalculation.
    ///
    /// Uses ssconvert with the `--recalc` and `-S` flags to export all sheets
    /// as separate CSV files. Returns paths to all generated CSV files.
    ///
    /// # Errors
    ///
    /// Returns an error if the xlsx path has no file stem, ssconvert fails to
    /// run, or ssconvert exits with a non-zero status.
    pub fn xlsx_to_csv(&self, xlsx_path: &Path, output_dir: &Path) -> Result<PathBuf, String> {
        let base_name = xlsx_path
            .file_stem()
            .ok_or("Invalid xlsx path: no file stem")?
            .to_string_lossy()
            .to_string();

        // Export all sheets with -S flag, using %n for sheet number
        let csv_pattern = output_dir.join(format!("{base_name}_%n.csv"));

        let output = Command::new(&self.path)
            .arg("--recalc")
            .arg("-S")
            .arg(xlsx_path)
            .arg(&csv_pattern)
            .output()
            .map_err(|e| format!("Failed to run ssconvert: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "ssconvert failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Return pattern path - caller will handle finding the right sheet
        Ok(output_dir.join(format!("{base_name}_")))
    }

    /// Converts XLSX to CSV files (all sheets) and returns all CSV paths.
    ///
    /// # Errors
    ///
    /// Returns an error if the xlsx path has no file stem, ssconvert fails,
    /// or no CSV files are generated.
    pub fn xlsx_to_csv_all_sheets(
        &self,
        xlsx_path: &Path,
        output_dir: &Path,
    ) -> Result<Vec<PathBuf>, String> {
        let base_name = xlsx_path
            .file_stem()
            .ok_or("Invalid xlsx path: no file stem")?
            .to_string_lossy()
            .to_string();

        // Export all sheets with -S flag
        let csv_pattern = output_dir.join(format!("{base_name}_%n.csv"));

        let output = Command::new(&self.path)
            .arg("--recalc")
            .arg("-S")
            .arg(xlsx_path)
            .arg(&csv_pattern)
            .output()
            .map_err(|e| format!("Failed to run ssconvert: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "ssconvert failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        // Find all generated CSV files
        let mut csv_files = Vec::new();
        for i in 0..10 {
            // Check up to 10 sheets
            let csv_path = output_dir.join(format!("{base_name}_{i}.csv"));
            if csv_path.exists() {
                csv_files.push(csv_path);
            } else {
                break;
            }
        }

        if csv_files.is_empty() {
            Err("No CSV files generated".to_string())
        } else {
            Ok(csv_files)
        }
    }
}

/// `LibreOffice` Calc, run headless, for the cases Gnumeric cannot read.
///
/// It runs with a profile of its own that recalculates every formula when an
/// xlsx file is loaded: by default `LibreOffice` shows the results cached in the
/// file, which are Forge's own.
pub struct LibreOfficeEngine {
    version: String,
    /// The profile directory (kept for the engine's lifetime)
    profile: tempfile::TempDir,
}

/// Recalculate every formula on load, for xlsx and ODF files.
const RECALC_ON_LOAD: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<oor:items xmlns:oor="http://openoffice.org/2001/registry" xmlns:xs="http://www.w3.org/2001/XMLSchema" xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance">
<item oor:path="/org.openoffice.Office.Calc/Formula/Load"><prop oor:name="OOXMLRecalcMode" oor:op="fuse"><value>0</value></prop></item>
<item oor:path="/org.openoffice.Office.Calc/Formula/Load"><prop oor:name="ODFRecalcMode" oor:op="fuse"><value>0</value></prop></item>
</oor:items>
"#;

/// The first version with `LET` (24.8).
pub const LIBREOFFICE_MINIMUM: (u32, u32) = (24, 8);

impl LibreOfficeEngine {
    /// Finds `soffice` and sets up its profile. `Err` says why it cannot be
    /// used: not installed, or older than [`LIBREOFFICE_MINIMUM`].
    ///
    /// # Errors
    ///
    /// Returns the reason `LibreOffice` cannot be used.
    pub fn detect() -> Result<Self, String> {
        let output = Command::new("soffice")
            .arg("--version")
            .output()
            .map_err(|e| format!("soffice not found ({e})"))?;
        let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let number = version
            .split_whitespace()
            .nth(1)
            .and_then(|v| {
                let mut parts = v.split('.').map(|p| p.parse::<u32>().ok());
                Some((parts.next()??, parts.next()??))
            })
            .ok_or_else(|| format!("cannot read the version of '{version}'"))?;
        if number < LIBREOFFICE_MINIMUM {
            return Err(format!(
                "{version} is older than {}.{}, the first with LET",
                LIBREOFFICE_MINIMUM.0, LIBREOFFICE_MINIMUM.1
            ));
        }
        let profile = tempfile::tempdir().map_err(|e| format!("profile directory: {e}"))?;
        let user = profile.path().join("user");
        std::fs::create_dir_all(&user).map_err(|e| format!("profile directory: {e}"))?;
        std::fs::write(user.join("registrymodifications.xcu"), RECALC_ON_LOAD)
            .map_err(|e| format!("profile: {e}"))?;
        Ok(Self { version, profile })
    }

    /// Returns the engine version string.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Recalculates `xlsx_path` and writes each sheet to a CSV file in
    /// `output_dir`, the values as numbers. Returns their paths.
    ///
    /// # Errors
    ///
    /// Returns an error if soffice fails or writes no CSV file.
    pub fn xlsx_to_csv_all_sheets(
        &self,
        xlsx_path: &Path,
        output_dir: &Path,
    ) -> Result<Vec<PathBuf>, String> {
        let out = output_dir.join("libreoffice");
        std::fs::create_dir_all(&out).map_err(|e| format!("output directory: {e}"))?;
        // CSV filter options: comma, double quote, UTF-8, from line 1, ...,
        // the last one (-1) writes every sheet to `{stem}-{sheet}.csv`
        let output = Command::new("soffice")
            .arg(format!(
                "-env:UserInstallation=file://{}",
                self.profile.path().display()
            ))
            .arg("--headless")
            .arg("--convert-to")
            .arg("csv:Text - txt - csv (StarCalc):44,34,76,1,,0,false,true,false,false,false,-1")
            .arg("--outdir")
            .arg(&out)
            .arg(xlsx_path)
            .output()
            .map_err(|e| format!("Failed to run soffice: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "soffice failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        let mut csv_files: Vec<PathBuf> = std::fs::read_dir(&out)
            .map_err(|e| format!("reading {}: {e}", out.display()))?
            .filter_map(|entry| entry.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|e| e == "csv"))
            .collect();
        csv_files.sort();
        if csv_files.is_empty() {
            Err(format!(
                "soffice wrote no CSV file: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ))
        } else {
            Ok(csv_files)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_name_is_constant() {
        assert_eq!(GnumericEngine::name(), "Gnumeric (ssconvert)");
    }

    #[test]
    fn engine_detection_returns_valid_engine_or_none() {
        let _ = GnumericEngine::detect();
    }
}
