// During coverage builds, stubbed main doesn't use imports
#![cfg_attr(coverage, allow(unused_imports))]

use mollendorff_forge::api::{run_api_server, server::ApiConfig};
use mollendorff_forge::cli;
use mollendorff_forge::error::{ForgeError, ForgeResult};

mod main_args;
use clap::Parser;
use main_args::{Cli, Commands};

/// CLI entry point - excluded from coverage (ADR-006)
/// Parses CLI args and dispatches to library functions.
/// Cannot unit test: reads from `std::env::args()`
/// Tested via: `cli_integration_tests.rs`
#[cfg(not(coverage))]
fn main() {
    // Print the readable message, not the Debug form a returned Result would give.
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

#[cfg(not(coverage))]
#[allow(clippy::too_many_lines)] // CLI dispatch function — splitting would fragment command handling
fn run() -> ForgeResult<()> {
    let cli = Cli::parse();

    // `forge functions | head` must end quietly, like other command-line tools.
    // The servers keep Rust's default, so a client that disconnects is an
    // error on that connection, not the end of the process.
    #[cfg(unix)]
    if !matches!(cli.command, Commands::Mcp { .. } | Commands::Serve { .. }) {
        // SAFETY: restoring the default action for SIGPIPE before any output.
        unsafe {
            libc::signal(libc::SIGPIPE, libc::SIG_DFL);
        }
    }

    match cli.command {
        Commands::Calculate {
            file,
            dry_run,
            verbose,
            scenario,
        } => cli::calculate(&file, dry_run, verbose, scenario.as_deref()),

        Commands::Audit { file, variable } => cli::audit(&file, &variable),

        Commands::Validate { files } => cli::validate(&files),

        Commands::Export {
            input,
            output,
            verbose,
        } => cli::export(&input, &output, verbose),

        Commands::Import {
            input,
            output,
            verbose,
            split_files,
            multi_doc,
        } => cli::import(&input, &output, verbose, split_files, multi_doc),

        Commands::Watch {
            file,
            validate,
            verbose,
        } => cli::watch(&file, validate, verbose),

        Commands::Compare {
            file,
            scenarios,
            format,
            verbose,
        } => cli::compare(
            &file,
            &scenarios,
            verbose,
            format.parse::<cli::ReportFormat>()?,
        ),

        Commands::Report {
            file,
            group,
            keys,
            format,
            no_inputs,
        } => cli::report(
            &file,
            &cli::ReportOptions {
                groups: group,
                keys,
                no_inputs,
            },
            format.parse::<cli::ReportFormat>()?,
        ),

        Commands::Variance {
            budget,
            actual,
            threshold,
            output,
            fail_on_alert,
            verbose,
        } => cli::variance(
            &budget,
            &actual,
            threshold,
            output.as_deref(),
            (fail_on_alert, verbose),
        ),

        Commands::Sensitivity {
            file,
            vary,
            range,
            vary2,
            range2,
            output_var,
            format,
            verbose,
        } => cli::sensitivity_in_format(
            &file,
            (&vary, &range),
            (vary2.as_deref(), range2.as_deref()),
            &output_var,
            (&format, verbose),
        ),

        Commands::GoalSeek {
            file,
            target,
            value,
            vary,
            min,
            max,
            tolerance,
            verbose,
        } => cli::goal_seek(&file, &target, value, &vary, (min, max), tolerance, verbose),

        Commands::BreakEven {
            file,
            output_var,
            vary,
            min,
            max,
            verbose,
        } => cli::break_even(&file, &output_var, &vary, min, max, verbose),

        Commands::Simulate {
            file,
            iterations,
            seed,
            sampling,
            output,
            verbose,
            scenario,
            seek,
            vary,
            min,
            max,
            output_var,
        } => {
            let options = cli::SimulateOptions {
                iterations,
                seed,
                sampling: sampling.as_deref(),
                scenario: scenario.as_deref(),
                outputs: &output_var,
            };
            match (seek, vary, min, max) {
                (Some(seek), Some(vary), Some(min), Some(max)) => {
                    cli::probability_seek(&file, options, &seek, &vary, min, max)
                },
                _ => cli::simulate(&file, options, output, verbose),
            }
        },

        Commands::Scenarios {
            file,
            scenario,
            output,
            verbose,
        } => cli::scenarios(&file, scenario.as_deref(), output, verbose),

        Commands::DecisionTree {
            file,
            dot,
            output,
            verbose,
        } => cli::decision_tree(&file, dot, output, verbose),

        Commands::RealOptions {
            file,
            option,
            compare_npv,
            output,
            verbose,
        } => cli::real_options(&file, option.as_deref(), compare_npv, output, verbose),

        Commands::Tornado {
            file,
            output_var,
            output,
            verbose,
        } => cli::tornado(&file, output_var.as_deref(), output, verbose),

        Commands::Bootstrap {
            file,
            iterations,
            seed,
            confidence,
            output,
            verbose,
        } => cli::bootstrap(&file, iterations, seed, confidence, output, verbose),

        Commands::Bayesian {
            file,
            query,
            evidence,
            output,
            verbose,
        } => cli::bayesian(&file, query.as_deref(), &evidence, output, verbose),

        Commands::Functions { json } => cli::functions(json),

        Commands::Schema { version, list } => cli::schema(version.as_deref(), list),

        Commands::Examples { name, run, json } => cli::examples(name, run, json),

        Commands::Upgrade {
            file,
            dry_run,
            to,
            verbose,
        } => cli::upgrade(&file, dry_run, &to, verbose),

        Commands::Update { check, verbose } => cli::update(check, verbose),

        Commands::Mcp { root, any_path } => {
            // Paths stay inside one folder unless --any-path (T1-MCP-ROOT-DEFAULT)
            let root = if any_path {
                None
            } else if let Some(dir) = root {
                Some(dir)
            } else {
                Some(std::env::current_dir().map_err(|e| {
                    ForgeError::Validation(format!(
                        "cannot read the current folder for --root: {e}"
                    ))
                })?)
            };
            mollendorff_forge::mcp::run_stdio(root.as_deref())
        },

        Commands::Serve {
            host,
            port,
            token,
            cors_origins,
            root,
        } => {
            let config = ApiConfig {
                host,
                port,
                token,
                cors_origins,
                root,
            };
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| ForgeError::Validation(format!("Failed to create runtime: {e}")))?;
            rt.block_on(run_api_server(config))
                .map_err(|e| ForgeError::Validation(format!("Server error: {e}")))?;
            Ok(())
        },
    }
}

/// Stub for coverage builds - see ADR-006
#[cfg(coverage)]
fn main() -> ForgeResult<()> {
    Ok(())
}
