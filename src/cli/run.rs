//! `forge` run from its arguments, in-process: the command line's whole
//! behaviour, from parsing the arguments to the exit code, on whatever
//! console it is given. `main.rs` gives it the process's own streams; a test
//! gives it buffers and reads exactly what a user would see.

use super::args::{
    AuditArgs, BayesianArgs, BootstrapArgs, BreakEvenArgs, CalculateArgs, Cli, Commands,
    CompareArgs, DecisionTreeArgs, ExamplesArgs, ExportArgs, FunctionsArgs, GoalSeekArgs,
    ImportArgs, McpArgs, RealOptionsArgs, ReportArgs, ScenariosArgs, SchemaArgs, SensitivityArgs,
    ServeArgs, SimulateArgs, TornadoArgs, UpdateArgs, UpgradeArgs, ValidateArgs, VarianceArgs,
    WatchArgs,
};
use super::console::{errln, out};
use super::Console;
use crate::error::{ForgeError, ForgeResult};
use clap::Parser;
use std::ffi::OsString;
use std::path::PathBuf;

/// Run `forge` with `args`, the program's name first as in `std::env::args`.
///
/// Returns the exit code: 0 when it succeeds, 1 when the command fails, with
/// `Error: ` and the reason on `err`, and 2 when the arguments are wrong. The
/// help and the version are written to `out` and exit 0.
pub fn run_from<I, T>(args: I, c: &mut Console<'_>) -> u8
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let code = match Cli::try_parse_from(args) {
        Ok(cli) => match cli.command.run(c) {
            Ok(()) => 0,
            Err(e) => {
                errln!(c, "Error: {e}");
                1
            },
        },
        Err(e) => usage(&e, c),
    };
    // A write that failed (a full disk, a closed pipe) fails the command too
    match c.finish() {
        Ok(()) => code,
        Err(e) => {
            errln!(c, "Error: {e}");
            // Nothing is left to report a failure to write that to.
            let _ = c.finish();
            1
        },
    }
}

/// Whether `args` start a server (`forge mcp`, `forge serve`).
///
/// A server must keep running when a client goes away; every other command
/// ends quietly when the program reading its output does (`forge functions |
/// head`).
pub fn starts_a_server<I, T>(args: I) -> bool
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    Cli::try_parse_from(args)
        .is_ok_and(|cli| matches!(cli.command, Commands::Mcp(_) | Commands::Serve(_)))
}

/// Write clap's help, version or usage error where clap would, and return
/// its exit code.
fn usage(e: &clap::Error, c: &mut Console<'_>) -> u8 {
    let render = |color: bool| {
        if color {
            e.render().ansi().to_string()
        } else {
            e.render().to_string()
        }
    };
    if e.use_stderr() {
        let text = render(c.color_err);
        c.write_err(format_args!("{text}"));
    } else {
        let text = render(c.color_out);
        out!(c, "{text}");
    }
    u8::try_from(e.exit_code()).unwrap_or(2)
}

impl Commands {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        match self {
            Self::Calculate(a) => a.run(c),
            Self::Audit(a) => a.run(c),
            Self::Validate(a) => a.run(c),
            Self::Export(a) => a.run(c),
            Self::Import(a) => a.run(c),
            Self::Watch(a) => a.run(c),
            Self::Compare(a) => a.run(c),
            Self::Report(a) => a.run(c),
            Self::Variance(a) => a.run(c),
            Self::Sensitivity(a) => a.run(c),
            Self::GoalSeek(a) => a.run(c),
            Self::BreakEven(a) => a.run(c),
            Self::Simulate(a) => a.run(c),
            Self::Scenarios(a) => a.run(c),
            Self::DecisionTree(a) => a.run(c),
            Self::RealOptions(a) => a.run(c),
            Self::Tornado(a) => a.run(c),
            Self::Bootstrap(a) => a.run(c),
            Self::Bayesian(a) => a.run(c),
            Self::Functions(a) => a.run(c),
            Self::Schema(a) => a.run(c),
            Self::Examples(a) => a.run(c),
            Self::Upgrade(a) => a.run(c),
            Self::Update(a) => a.run(c),
            Self::Mcp(a) => a.run(),
            Self::Serve(a) => a.run(),
        }
    }
}

impl CalculateArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::calculate(
            c,
            &self.file,
            self.dry_run,
            self.verbose,
            self.scenario.as_deref(),
        )
    }
}

impl AuditArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::audit(c, &self.file, &self.variable)
    }
}

impl ValidateArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::validate(c, &self.files)
    }
}

impl ExportArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::export(c, &self.input, &self.output, self.verbose)
    }
}

impl ImportArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::import(
            c,
            &self.input,
            &self.output,
            self.verbose,
            self.split_files,
            self.multi_doc,
        )
    }
}

impl WatchArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::watch(c, &self.file, self.validate, self.verbose)
    }
}

impl CompareArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::compare(c, &self.file, &self.scenarios, self.verbose, self.format)
    }
}

impl ReportArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        let options = super::ReportOptions {
            groups: self.group,
            keys: self.keys,
            no_inputs: self.no_inputs,
        };
        super::report(c, &self.file, &options, self.format)
    }
}

impl VarianceArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::variance(
            c,
            &self.budget,
            &self.actual,
            self.threshold,
            self.output.as_deref(),
            (self.fail_on_alert, self.verbose),
        )
    }
}

impl SensitivityArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::sensitivity_in_format(
            c,
            &self.file,
            (&self.vary, &self.range),
            (self.vary2.as_deref(), self.range2.as_deref()),
            &self.output_var,
            (&self.format, self.verbose),
        )
    }
}

impl GoalSeekArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::goal_seek(
            c,
            &self.file,
            (&self.target, self.value),
            &self.vary,
            (self.min, self.max),
            self.tolerance,
            self.verbose,
        )
    }
}

impl BreakEvenArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::break_even(
            c,
            &self.file,
            &self.output_var,
            &self.vary,
            self.min,
            self.max,
            self.verbose,
        )
    }
}

impl SimulateArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        let options = super::SimulateOptions {
            iterations: self.iterations,
            seed: self.seed,
            sampling: self.sampling.as_deref(),
            scenario: self.scenario.as_deref(),
            outputs: &self.output_var,
        };
        match (self.seek, self.vary, self.min, self.max) {
            (Some(seek), Some(vary), Some(min), Some(max)) => {
                super::probability_seek(c, &self.file, options, &seek, &vary, min, max)
            },
            _ => super::simulate(c, &self.file, options, self.output, self.verbose),
        }
    }
}

impl ScenariosArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::scenarios(
            c,
            &self.file,
            self.scenario.as_deref(),
            self.output,
            self.verbose,
        )
    }
}

impl DecisionTreeArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::decision_tree(c, &self.file, self.dot, self.output, self.verbose)
    }
}

impl RealOptionsArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::real_options(
            c,
            &self.file,
            self.option.as_deref(),
            self.compare_npv,
            self.output,
            self.verbose,
        )
    }
}

impl TornadoArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::tornado(
            c,
            &self.file,
            self.output_var.as_deref(),
            self.output,
            self.verbose,
        )
    }
}

impl BootstrapArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::bootstrap(
            c,
            &self.file,
            self.iterations,
            self.seed,
            self.confidence,
            self.output,
            self.verbose,
        )
    }
}

impl BayesianArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::bayesian(
            c,
            &self.file,
            self.query.as_deref(),
            &self.evidence,
            self.output,
            self.verbose,
        )
    }
}

impl FunctionsArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::functions(c, self.json)
    }
}

impl SchemaArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::schema(c, self.version.as_deref(), self.list)
    }
}

impl ExamplesArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::examples(c, self.name, self.run, self.json)
    }
}

impl UpgradeArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::upgrade(c, &self.file, self.dry_run, &self.to, self.verbose)
    }
}

impl UpdateArgs {
    fn run(self, c: &mut Console<'_>) -> ForgeResult<()> {
        super::update(c, self.check, self.verbose)
    }
}

impl McpArgs {
    fn run(self) -> ForgeResult<()> {
        self.root(std::env::current_dir)
            .and_then(|root| crate::mcp::run_stdio(root.as_deref()))
    }

    /// The folder file paths must stay inside: `--root`, or else the folder
    /// the server starts in, unless `--any-path` (T1-MCP-ROOT-DEFAULT).
    fn root(
        self,
        current_dir: impl FnOnce() -> std::io::Result<PathBuf>,
    ) -> ForgeResult<Option<PathBuf>> {
        if self.any_path {
            return Ok(None);
        }
        self.root.map_or_else(
            || {
                current_dir().map(Some).map_err(|e| {
                    ForgeError::Validation(format!(
                        "cannot read the current folder for --root: {e}"
                    ))
                })
            },
            |dir| Ok(Some(dir)),
        )
    }
}

impl ServeArgs {
    fn run(self) -> ForgeResult<()> {
        crate::api::serve(crate::api::server::ApiConfig {
            host: self.host,
            port: self.port,
            token: self.token,
            cors_origins: self.cors_origins,
            root: self.root,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::console::{capture, Captured};

    const BAGEL: &str = "examples/bagel/bagel.yaml";

    /// `forge` with these arguments, in-process: its exit code and what it
    /// wrote.
    fn forge(args: &[&str]) -> (u8, Captured) {
        capture(|c| run_from(std::iter::once("forge").chain(args.iter().copied()), c))
    }

    /// `forge` that must succeed: what it printed.
    fn ok(args: &[&str]) -> String {
        let (code, printed) = forge(args);
        assert_eq!(code, 0, "{args:?}: {}", printed.both());
        printed.out
    }

    /// `forge` that must fail with exit code 1: its message.
    fn fails(args: &[&str]) -> String {
        let (code, printed) = forge(args);
        assert_eq!(code, 1, "{args:?}: {}", printed.both());
        assert!(printed.err.starts_with("Error: "), "{}", printed.err);
        printed.err
    }

    fn first_line(text: &str) -> &str {
        text.lines().next().unwrap_or_default()
    }

    #[test]
    fn help_and_version_go_to_out_and_exit_zero() {
        let help = ok(&["--help"]);
        assert!(
            help.starts_with("Forge - Git-native financial modeling\n"),
            "{help}"
        );
        assert!(help.contains("\nUsage: forge <COMMAND>\n"), "{help}");
        let version = ok(&["--version"]);
        assert!(
            version.starts_with(&format!("forge {}", env!("CARGO_PKG_VERSION"))),
            "{version}"
        );
        let sub = ok(&["calculate", "--help"]);
        assert!(
            sub.contains("Usage: forge calculate [OPTIONS] <FILE>"),
            "{sub}"
        );
    }

    #[test]
    fn a_wrong_argument_is_a_usage_error_on_err_with_exit_code_two() {
        for args in [&["nope"][..], &[], &["calculate", "--bogus", "x.yaml"]] {
            let (code, printed) = forge(args);
            assert_eq!(code, 2, "{args:?}");
            assert_eq!(printed.out, "", "{args:?}");
            assert!(
                printed.err.contains("Usage: forge"),
                "{args:?}: {}",
                printed.err
            );
        }
        let (_, printed) = forge(&["nope"]);
        assert!(printed
            .err
            .starts_with("error: unrecognized subcommand 'nope'"));
    }

    #[test]
    fn help_and_errors_are_in_colour_when_the_console_says_so() {
        let (mut out, mut err, mut input) = (Vec::new(), Vec::new(), std::io::empty());
        let mut c = Console::new(&mut out, &mut err, &mut input);
        c.color_out = true;
        c.color_err = true;
        assert_eq!(run_from(["forge", "--help"], &mut c), 0);
        assert_eq!(run_from(["forge", "nope"], &mut c), 2);
        drop(c);
        assert!(String::from_utf8(out).unwrap().contains("\x1b["));
        assert!(String::from_utf8(err).unwrap().contains("\x1b["));
    }

    #[test]
    fn a_failing_command_says_why_with_exit_code_one() {
        let err = fails(&["calculate", "missing.yaml"]);
        assert!(err.contains("No such file or directory"), "{err}");
    }

    #[test]
    fn a_write_that_fails_fails_the_command() {
        struct Full;
        impl std::io::Write for Full {
            fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("disk full"))
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let (mut out, mut err, mut input) = (Full, Vec::new(), std::io::empty());
        let mut c = Console::new(&mut out, &mut err, &mut input);
        assert_eq!(run_from(["forge", "functions"], &mut c), 1);
        drop(c);
        assert_eq!(String::from_utf8(err).unwrap(), "Error: disk full\n");
    }

    #[test]
    fn only_mcp_and_serve_start_a_server() {
        assert!(starts_a_server(["forge", "mcp"]));
        assert!(starts_a_server(["forge", "serve"]));
        assert!(!starts_a_server(["forge", "functions"]));
        assert!(!starts_a_server(["forge", "nope"]));
    }

    #[test]
    fn each_model_command_runs() {
        let calculated = ok(&["calculate", BAGEL, "--dry-run"]);
        assert_eq!(first_line(&calculated), "🔥 Forge - Calculating formulas");
        assert!(calculated.ends_with("📋 Dry run complete - no changes written\n"));
        let audit = ok(&["audit", BAGEL, "year.profit"]);
        assert_eq!(first_line(&audit), "🔍 Forge - Audit Trail");
        assert!(audit.contains("year.profit"), "{audit}");
        let valid = ok(&["validate", BAGEL]);
        assert_eq!(first_line(&valid), "✅ Validating model");
        let report = ok(&["report", BAGEL, "--format", "csv"]);
        assert!(report.starts_with("group,"), "{report}");
        let compared = ok(&[
            "compare",
            BAGEL,
            "--scenarios",
            "base,flour_shock",
            "--format",
            "json",
        ]);
        let json: serde_json::Value = serde_json::from_str(&compared).unwrap();
        assert!(json.to_string().contains("flour_shock"), "{json}");
        let err = fails(&["watch", "missing.yaml"]);
        assert!(err.contains("File not found: missing.yaml"), "{err}");
        let upgraded = ok(&["upgrade", "examples/starter.yaml", "--dry-run"]);
        assert_eq!(first_line(&upgraded), "🔥 Forge - Schema Upgrade");
    }

    #[test]
    fn export_then_import_through_the_command_line() {
        let dir = tempfile::tempdir().unwrap();
        let xlsx = dir.path().join("bagel.xlsx");
        let yaml = dir.path().join("back.yaml");
        let (xlsx_arg, yaml_arg) = (xlsx.to_str().unwrap(), yaml.to_str().unwrap());
        assert_eq!(
            first_line(&ok(&["export", BAGEL, xlsx_arg])),
            "🔥 Forge - Excel Export"
        );
        let imported = ok(&["import", xlsx_arg, yaml_arg]);
        assert_eq!(first_line(&imported), "🔥 Forge - Excel Import");
        assert!(std::fs::read_to_string(&yaml)
            .unwrap()
            .contains("price_per_dozen"));
    }

    #[test]
    fn each_analysis_command_runs() {
        let variance = ok(&[
            "variance",
            "examples/household/budget.yaml",
            "examples/household/actual.yaml",
        ]);
        assert_eq!(first_line(&variance), "🔥 Forge - Variance Analysis");
        let sensitivity = ok(&[
            "sensitivity",
            "examples/periods/dcf.yaml",
            "--vary",
            "assumptions.wacc",
            "-r",
            "0.08,0.11,0.005",
            "--output-var",
            "valuation.equity_value",
        ]);
        assert_eq!(first_line(&sensitivity), "🔥 Forge - Sensitivity Analysis");
        let seek = ok(&[
            "goal-seek",
            BAGEL,
            "--target",
            "year.profit",
            "--value",
            "60000",
            "--vary",
            "assumptions.staff_hours_per_day",
        ]);
        assert_eq!(first_line(&seek), "🔥 Forge - Goal Seek");
        let even = ok(&[
            "break-even",
            BAGEL,
            "--output-var",
            "year.profit",
            "--vary",
            "assumptions.price_per_dozen",
        ]);
        assert_eq!(first_line(&even), "🔥 Forge - Break-Even Analysis");
    }

    #[test]
    fn each_uncertainty_command_runs() {
        let simulate = "examples/bagel/bagel_simulate.yaml";
        let simulated = ok(&[
            "simulate",
            simulate,
            "--iterations",
            "200",
            "--output-var",
            "year.profit",
        ]);
        assert_eq!(first_line(&simulated), "🎲 Forge - Monte Carlo Simulation");
        let err = fails(&[
            "simulate",
            simulate,
            "--iterations",
            "200",
            "--seek",
            "P(year.profit > 0) = 0.5",
            "--vary",
            "assumptions.price_per_dozen",
            "--min",
            "20",
            "--max",
            "40",
        ]);
        assert!(
            err.contains("the goal 50.00% is not crossed in that range"),
            "{err}"
        );
        for (args, header) in [
            (&["scenarios", BAGEL][..], "📊 Forge - Scenario Analysis"),
            (
                &["decision-tree", "examples/bagel/second_shop.yaml"],
                "🌳 Forge - Decision Tree Analysis",
            ),
            (
                &["real-options", "examples/bagel/wait_option.yaml"],
                "💎 Forge - Real Options Analysis",
            ),
            (&["tornado", BAGEL], "🌪️ Forge - Tornado Diagram"),
            (
                &["bootstrap", "examples/bagel/first_month.yaml"],
                "🔄 Forge - Bootstrap Resampling",
            ),
            (
                &[
                    "bayesian",
                    "examples/bagel/slow_week.yaml",
                    "-q",
                    "roadwork",
                ],
                "🔮 Forge - Bayesian Network Inference",
            ),
        ] {
            assert_eq!(first_line(&ok(args)), header, "{args:?}");
        }
    }

    #[test]
    fn each_reference_command_runs() {
        assert_eq!(
            first_line(&ok(&["functions"])),
            "🔥 Forge - Supported Functions"
        );
        let schema: serde_json::Value = serde_json::from_str(&ok(&["schema", "v5"])).unwrap();
        assert!(schema.is_object());
        assert_eq!(first_line(&ok(&["examples", "starter"])), "# starter");
    }

    #[test]
    fn the_servers_refuse_what_they_cannot_serve() {
        let err = fails(&["mcp", "--root", "nowhere-at-all"]);
        assert!(err.contains("--root nowhere-at-all"), "{err}");
        let err = fails(&["serve", "--host", "0.0.0.0"]);
        assert!(
            err.contains("Refusing to listen on 0.0.0.0 without a token"),
            "{err}"
        );
    }

    #[test]
    fn the_mcp_root_is_the_one_given_or_the_current_folder() {
        let args = |root: Option<&str>, any_path| McpArgs {
            root: root.map(PathBuf::from),
            any_path,
        };
        let here = || Ok(PathBuf::from("/here"));
        assert_eq!(
            args(Some("/given"), false).root(here).unwrap(),
            Some(PathBuf::from("/given"))
        );
        assert_eq!(
            args(None, false).root(here).unwrap(),
            Some(PathBuf::from("/here"))
        );
        assert_eq!(args(Some("/given"), true).root(here).unwrap(), None);
        let gone = || Err(std::io::Error::other("folder removed"));
        let err = args(None, false).root(gone).unwrap_err().to_string();
        assert!(
            err.contains("cannot read the current folder for --root: folder removed"),
            "{err}"
        );
    }
}
