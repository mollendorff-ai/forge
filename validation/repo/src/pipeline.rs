//! The guide's workflow for a repository of models does what its page says
//! (roadmap A6-GUIDE-PIPELINE, ADR-078).
//!
//! `examples/pipeline/forge.yml` is the workflow `docs/guide/pipeline.md`
//! has a reader copy. Its steps run here as GitHub Actions runs them: each
//! with `bash --noprofile --norc -eo pipefail`, in order, stopping at the
//! first that fails, in a folder holding lesson 2's model, with the `forge`
//! of these sources first on `PATH` in place of the release the install step
//! downloads. It passes when every result is stored and up to date, and fails
//! at the step the page names when a result is out of date and when one was
//! never calculated.

use crate::Ran;
use serde_json::Value;
use std::path::Path;
use std::process::Command;

/// The workflow, by its path from the top.
pub const WORKFLOW: &str = "examples/pipeline/forge.yml";

/// A step of the job that runs commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// What the run's page calls it
    pub name: String,
    /// The script it runs
    pub run: String,
}

/// The steps of `workflow`'s job `check` that have a name and a script, in
/// order: a step that uses an action is left out.
///
/// # Panics
///
/// Panics if the workflow is not YAML or has no job `check` with steps.
#[must_use]
pub fn steps(workflow: &str) -> Vec<Step> {
    let parsed: Value = yaml_serde::from_str(workflow).expect("a workflow in YAML");
    parsed["jobs"]["check"]["steps"]
        .as_array()
        .expect("a job check with steps")
        .iter()
        .filter_map(|step| {
            Some(Step {
                name: step["name"].as_str()?.to_string(),
                run: step["run"].as_str()?.to_string(),
            })
        })
        .collect()
}

/// What a step printed, and how it ended.
#[derive(Debug)]
pub struct StepRun {
    /// The step's name
    pub name: String,
    /// Its exit code and output
    pub ran: Ran,
}

/// Run `steps` in `dir` as the runner runs them, with `bin` first on `PATH`
/// and `GITHUB_STEP_SUMMARY` set to `summary`, until one fails.
///
/// # Panics
///
/// Panics if `bash` cannot start.
#[must_use]
pub fn run_steps(steps: &[Step], dir: &Path, bin: &Path, summary: &Path) -> Vec<StepRun> {
    let path = format!(
        "{}:{}",
        bin.display(),
        std::env::var("PATH").unwrap_or_default()
    );
    let mut runs = Vec::new();
    for step in steps {
        let out = Command::new("bash")
            .args(["--noprofile", "--norc", "-eo", "pipefail", "-c", &step.run])
            .current_dir(dir)
            .env("PATH", &path)
            .env("GITHUB_STEP_SUMMARY", summary)
            .env("NO_COLOR", "1")
            .output()
            .expect("bash starts");
        let ran = Ran {
            code: out.status.code(),
            out: String::from_utf8_lossy(&out.stdout).into_owned(),
            err: String::from_utf8_lossy(&out.stderr).into_owned(),
        };
        let failed = !ran.success();
        runs.push(StepRun {
            name: step.name.clone(),
            ran,
        });
        if failed {
            break;
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{forge_bin, forge_in, read};
    use std::path::PathBuf;

    /// A repository of models as lesson 3 leaves it, the binary to put on
    /// `PATH`, and the file the run's page is written to.
    struct Repository {
        _dir: tempfile::TempDir,
        models: PathBuf,
        bin: PathBuf,
        summary: PathBuf,
    }

    impl Repository {
        /// Lesson 2's `bagel.yaml` and `costs.yaml`, calculated as `calculate`
        /// names them (`bagel.yaml` alone, or both).
        fn with(calculated: &[&str]) -> Self {
            let dir = tempfile::tempdir().expect("a temporary folder");
            let models = dir.path().join("models");
            let bin = dir.path().join("bin");
            std::fs::create_dir_all(&models).expect("a folder for the models");
            std::fs::create_dir_all(&bin).expect("a folder for forge");
            std::os::unix::fs::symlink(forge_bin(), bin.join("forge")).expect("forge on PATH");
            for (to, from) in [
                ("bagel.yaml", "02-bagel.yaml"),
                ("costs.yaml", "costs.yaml"),
            ] {
                std::fs::write(models.join(to), read(&format!("examples/guide/{from}")))
                    .expect("a model to write");
            }
            for model in calculated {
                let printed = forge_in(&models, &["calculate", model]).printed();
                assert!(
                    printed.contains(&format!("updated in {model}")),
                    "{printed}"
                );
            }
            let summary = dir.path().join("summary.md");
            Self {
                _dir: dir,
                models,
                bin,
                summary,
            }
        }

        /// The workflow's steps after the install, run here.
        fn check(&self) -> Vec<StepRun> {
            let steps: Vec<Step> = steps(&read(WORKFLOW))
                .into_iter()
                .filter(|step| step.name != "Install Forge")
                .collect();
            run_steps(&steps, &self.models, &self.bin, &self.summary)
        }
    }

    /// The names of the steps that ran, and the exit code of the last.
    fn ended(runs: &[StepRun]) -> (Vec<&str>, Option<i32>) {
        let names = runs.iter().map(|r| r.name.as_str()).collect();
        (names, runs.last().and_then(|r| r.ran.code))
    }

    const VALIDATE: &str = "Every stored result is what its formula gives";
    const JSON: &str = "The results, as JSON";
    const SUMMARY: &str = "The results, on the run's page";

    #[test]
    fn the_steps_are_the_named_scripts_in_order() {
        let workflow = "jobs:\n  check:\n    steps:\n      - uses: actions/checkout@v7\n      - name: One\n        run: echo one\n      - name: Upload\n        uses: actions/upload-artifact@v7\n      - name: Two\n        run: |\n          echo two\n          echo three\n";
        assert_eq!(
            steps(workflow),
            [
                Step {
                    name: "One".to_string(),
                    run: "echo one".to_string()
                },
                Step {
                    name: "Two".to_string(),
                    run: "echo two\necho three\n".to_string()
                },
            ]
        );
    }

    #[test]
    fn the_steps_stop_at_the_first_that_fails() {
        let dir = tempfile::tempdir().expect("a temporary folder");
        let step = |name: &str, run: &str| Step {
            name: name.to_string(),
            run: run.to_string(),
        };
        let summary = dir.path().join("summary.md");
        let runs = run_steps(
            &[
                step("Write", "echo written >> \"$GITHUB_STEP_SUMMARY\""),
                step("Fail in a pipe", "false | cat"),
                step("Never", "echo never"),
            ],
            dir.path(),
            dir.path(),
            &summary,
        );
        assert_eq!(ended(&runs), (vec!["Write", "Fail in a pipe"], Some(1)));
        assert_eq!(
            std::fs::read_to_string(&summary).expect("a summary"),
            "written\n"
        );
    }

    #[test]
    fn the_install_step_installs_a_release() {
        let workflow = read(WORKFLOW);
        let parsed: Value = yaml_serde::from_str(&workflow).expect("a workflow in YAML");
        let version = parsed["env"]["FORGE_VERSION"]
            .as_str()
            .expect("FORGE_VERSION");
        let release = format!("## [{}]", version.trim_start_matches('v'));
        assert!(
            read("CHANGELOG.md").contains(&release),
            "{WORKFLOW} installs {version}, which CHANGELOG.md does not list as a release"
        );
        let install = steps(&workflow)
            .into_iter()
            .find(|step| step.name == "Install Forge")
            .expect("a step that installs Forge");
        assert!(install.run.contains(
            "https://raw.githubusercontent.com/mollendorff-ai/forge/$FORGE_VERSION/install.sh"
        ));
        assert!(install.run.contains("$GITHUB_PATH"));
    }

    #[test]
    fn models_calculated_and_up_to_date_pass() {
        let repo = Repository::with(&["bagel.yaml", "costs.yaml"]);
        let runs = repo.check();
        assert_eq!(ended(&runs), (vec![VALIDATE, JSON, SUMMARY], Some(0)));
        let results: Value = serde_json::from_str(
            &std::fs::read_to_string(repo.models.join("results/bagel.json")).expect("results"),
        )
        .expect("results in JSON");
        let year = &results["sections"][1]["rows"][0];
        assert_eq!(year["group"], "year");
        assert_eq!(year["values"]["profit"], 22_247.057_611_775_4);
        let summary = std::fs::read_to_string(&repo.summary).expect("a summary");
        assert!(summary.contains("### bagel.yaml"), "{summary}");
        assert!(
            summary.contains("| year | 386,827.45 | 22,247.06 | 0.05751 | -6,402.79 |"),
            "{summary}"
        );
        assert!(summary.contains("### costs.yaml"), "{summary}");
    }

    #[test]
    fn a_result_out_of_date_fails_validate() {
        let repo = Repository::with(&["bagel.yaml", "costs.yaml"]);
        let edited = forge_in(
            &repo.models,
            &["edit", "bagel.yaml", "--set", "shop.price_per_dozen=17"],
        )
        .printed();
        assert!(edited.contains("Wrote bagel.yaml"), "{edited}");
        let runs = repo.check();
        assert_eq!(ended(&runs), (vec![VALIDATE], Some(1)));
        let (out, err) = (&runs[0].ran.out, &runs[0].ran.err);
        assert!(out.contains("❌ bagel.yaml - FAILED"), "{out}");
        let error = "Error: Validation error: 1 file(s) failed validation";
        assert!(err.contains(error), "{err}");
    }

    #[test]
    fn a_formula_never_calculated_fails_validate() {
        let repo = Repository::with(&["bagel.yaml"]);
        let runs = repo.check();
        assert_eq!(ended(&runs), (vec![VALIDATE], Some(1)));
        let out = &runs[0].ran.out;
        assert!(out.contains("1 passed, 1 failed out of 2 files"), "{out}");
        let named =
            "no stored result (ingredients.per_dozen, labour.loaded_wage): run forge calculate";
        assert!(out.contains(named), "{out}");
    }
}
