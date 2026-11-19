//! Watch mode: run `validate` or `calculate` again each time the model, or a
//! YAML file beside it, changes.

use super::reject_distributions;
use crate::cli::console::{errln, out, outln};
use crate::cli::Console;
use crate::core::ArrayCalculator;
use crate::error::{ForgeError, ForgeResult};
use crate::parser;
use colored::Colorize;
use notify::RecursiveMode;
use notify_debouncer_mini::{new_debouncer, DebounceEventResult, DebouncedEventKind, Debouncer};
use std::path::Path;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// Start watching `dir` (not its subdirectories), debouncing rapid-fire
/// events such as a save that writes a file twice.
fn start_watching(
    dir: &Path,
) -> ForgeResult<(
    Receiver<DebounceEventResult>,
    Debouncer<notify::RecommendedWatcher>,
)> {
    let (tx, rx) = channel();
    new_debouncer(Duration::from_millis(200), tx)
        .and_then(|mut debouncer| {
            debouncer
                .watcher()
                .watch(dir, RecursiveMode::NonRecursive)
                .map(|()| debouncer)
        })
        .map(|debouncer| (rx, debouncer))
        .map_err(|e| {
            ForgeError::Validation(format!("Failed to watch directory {}: {e}", dir.display()))
        })
}

/// Execute the watch command
///
/// # Errors
///
/// Returns an error if the file does not exist or the directory cannot be
/// watched. It returns nothing otherwise: it runs until the process stops.
pub fn watch(
    c: &mut Console<'_>,
    file: &Path,
    validate_only: bool,
    verbose: bool,
) -> ForgeResult<()> {
    watch_with(c, file, validate_only, verbose, start_watching)
}

/// The watch command, with what watches the directory given: the events it
/// sends, and the watcher that must stay alive while they come. It returns
/// when the events stop coming.
fn watch_with<W>(
    c: &mut Console<'_>,
    file: &Path,
    validate_only: bool,
    verbose: bool,
    start: impl FnOnce(&Path) -> ForgeResult<(Receiver<DebounceEventResult>, W)>,
) -> ForgeResult<()> {
    outln!(c, "{}", "👁️  Forge - Watch Mode".bold().green());
    outln!(c, "   Watching: {}", file.display());
    outln!(
        c,
        "   Mode: {}",
        if validate_only {
            "validate only"
        } else {
            "calculate"
        }
    );
    outln!(c, "   Press {} to stop\n", "Ctrl+C".bold().yellow());

    // The canonical path, which is also the check that the file exists
    let canonical_path = file
        .canonicalize()
        .map_err(|_| ForgeError::Validation(format!("File not found: {}", file.display())))?;
    let parent_dir = canonical_path
        .parent()
        .ok_or_else(|| ForgeError::Validation("Cannot determine parent directory".to_string()))?;

    // Watch the parent directory (all YAML files in that directory count)
    let (rx, _watcher) = start(parent_dir)?;

    if verbose {
        outln!(
            c,
            "   {} {}",
            "Watching directory:".cyan(),
            parent_dir.display()
        );
    }

    // Run initial validation/calculation
    outln!(c, "{}", "🔄 Initial run...".cyan());
    run_watch_action(c, file, validate_only, verbose);
    outln!(c);

    watch_loop(c, &rx, file, &canonical_path, validate_only, verbose);
    Ok(())
}

/// Run the action again for each batch of events that concerns the model,
/// until the events stop.
fn watch_loop(
    c: &mut Console<'_>,
    rx: &Receiver<DebounceEventResult>,
    file: &Path,
    canonical_path: &Path,
    validate_only: bool,
    verbose: bool,
) {
    loop {
        match rx.recv() {
            Ok(Ok(events)) => {
                if is_relevant(&events, canonical_path) {
                    // Clear screen for fresh output (verbose mode only)
                    if verbose {
                        out!(c, "\x1B[2J\x1B[1;1H"); // ANSI clear screen
                    }
                    outln!(
                        c,
                        "\n{} {}",
                        "🔄 Change detected at".cyan(),
                        chrono_lite_timestamp().cyan()
                    );
                    run_watch_action(c, file, validate_only, verbose);
                    outln!(c);
                }
            },
            Ok(Err(error)) => {
                errln!(c, "{} Watch error: {}", "❌".red(), error);
            },
            Err(e) => {
                errln!(c, "{} Channel error: {}", "❌".red(), e);
                break;
            },
        }
    }
}

/// Whether any of the events is a change to the model file or to a YAML file
/// beside it
fn is_relevant(events: &[notify_debouncer_mini::DebouncedEvent], canonical_path: &Path) -> bool {
    use std::ffi::OsStr;

    events.iter().any(|event| {
        event.kind == DebouncedEventKind::Any
            && (event.path.canonicalize().is_ok_and(|p| p == canonical_path)
                || event.path.file_name() == canonical_path.file_name()
                || event
                    .path
                    .extension()
                    .and_then(OsStr::to_str)
                    .is_some_and(|ext| {
                        ext.eq_ignore_ascii_case("yaml") || ext.eq_ignore_ascii_case("yml")
                    }))
    })
}

/// Get a simple timestamp without external dependencies
fn chrono_lite_timestamp() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let hours = (secs / 3600) % 24;
    let minutes = (secs / 60) % 60;
    let seconds = secs % 60;
    format!("{hours:02}:{minutes:02}:{seconds:02} UTC")
}

/// Run the watch action (validate or calculate)
fn run_watch_action(c: &mut Console<'_>, file: &Path, validate_only: bool, verbose: bool) {
    if validate_only {
        match validate_internal(c, file, verbose) {
            Ok(()) => outln!(c, "{}", "✅ Validation passed".bold().green()),
            Err(e) => outln!(c, "{} {}", "❌ Validation failed:".bold().red(), e),
        }
    } else {
        match calculate_internal(c, file, verbose) {
            Ok(()) => outln!(c, "{}", "✅ Calculation complete".bold().green()),
            Err(e) => outln!(c, "{} {}", "❌ Calculation failed:".bold().red(), e),
        }
    }
}

/// Internal validation function for watch mode
fn validate_internal(c: &mut Console<'_>, file: &Path, verbose: bool) -> ForgeResult<()> {
    const TOLERANCE: f64 = 0.0001;

    let model = parser::parse_model(file)?;
    reject_distributions(&model)?;

    if verbose {
        outln!(
            c,
            "   Found {} tables, {} scalars",
            model.tables.len(),
            model.scalars.len()
        );
    }

    // Note: Table column length validation is deferred to calculation time
    // Row-wise operations will validate at runtime in array_calculator

    // Calculate and compare
    let calculator = ArrayCalculator::new(model.clone());
    let calculated = calculator.calculate_all()?;

    // Check for mismatches
    let mut mismatches = Vec::new();

    for (var_name, original) in &model.scalars {
        let calculated_value = calculated.scalars.get(var_name).and_then(|v| v.value);
        if let (Some(current_value), Some(calculated_value)) = (original.value, calculated_value) {
            let diff = (current_value - calculated_value).abs();
            if diff > TOLERANCE {
                mismatches.push((var_name.clone(), current_value, calculated_value));
            }
        }
    }

    if !mismatches.is_empty() {
        let msg = mismatches
            .iter()
            .map(|(name, current, expected)| {
                format!("  {name} current={current} expected={expected}")
            })
            .collect::<Vec<_>>()
            .join("\n");
        return Err(ForgeError::Validation(format!(
            "{} value mismatches:\n{}",
            mismatches.len(),
            msg
        )));
    }

    Ok(())
}

/// Internal calculation function for watch mode
fn calculate_internal(c: &mut Console<'_>, file: &Path, verbose: bool) -> ForgeResult<()> {
    let model = parser::parse_model(file)?;

    if verbose {
        outln!(
            c,
            "   Found {} tables, {} scalars",
            model.tables.len(),
            model.scalars.len()
        );
    }

    let calculator = ArrayCalculator::new(model);
    let result = calculator.calculate_all()?;

    // Show summary
    for (table_name, table) in &result.tables {
        outln!(
            c,
            "   📊 {} ({} columns)",
            table_name.bright_blue(),
            table.columns.len()
        );
    }

    if !result.scalars.is_empty() && verbose {
        outln!(c, "   📐 {} scalars calculated", result.scalars.len());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::console::capture;
    use notify_debouncer_mini::DebouncedEvent;
    use std::fs;
    use std::path::PathBuf;

    /// A model whose stored value agrees with its formula
    const GOOD: &str = "_forge_version: \"5.0.0\"\ninputs:\n  a:\n    value: 2\n    formula: null\noutputs:\n  b:\n    value: 4\n    formula: \"=a * 2\"\nt:\n  n: [1, 2]\n  d: \"=n * 2\"\n";

    fn event(path: &str, kind: DebouncedEventKind) -> DebouncedEvent {
        DebouncedEvent::new(PathBuf::from(path), kind)
    }

    fn any(path: &str) -> DebouncedEvent {
        event(path, DebouncedEventKind::Any)
    }

    #[test]
    fn an_event_concerns_the_model_or_a_yaml_file_beside_it() {
        let dir = tempfile::tempdir().unwrap();
        let model = dir.path().join("m.yaml");
        fs::write(&model, GOOD).unwrap();
        let model = model.canonicalize().unwrap();
        let relevant = |event| is_relevant(&[event], &model);

        assert!(relevant(any(model.to_str().unwrap())));
        // The same name in another folder, a file that is gone
        assert!(relevant(any("/elsewhere/m.yaml")));
        assert!(relevant(any("/elsewhere/other.YAML")));
        assert!(relevant(any("/elsewhere/other.yml")));
        assert!(!relevant(any("/elsewhere/notes.txt")));
        assert!(!relevant(any("/")));
        // A change still going on is not reported until it settles
        assert!(!relevant(event(
            model.to_str().unwrap(),
            DebouncedEventKind::AnyContinuous
        )));
        assert!(!is_relevant(&[], &model));
        assert!(is_relevant(&[any("/x.txt"), any("/x.yaml")], &model));
    }

    #[test]
    fn a_directory_can_be_watched_and_one_that_is_missing_cannot() {
        let dir = tempfile::tempdir().unwrap();
        let (rx, _watcher) = start_watching(dir.path()).unwrap();
        assert!(rx.try_recv().is_err());
        let err = start_watching(&dir.path().join("absent"))
            .err()
            .unwrap()
            .to_string();
        assert!(err.contains("Failed to watch directory"), "{err}");
    }

    #[test]
    fn a_change_to_a_watched_directory_reaches_the_receiver() {
        let dir = tempfile::tempdir().unwrap();
        let (rx, _watcher) = start_watching(dir.path()).unwrap();
        fs::write(dir.path().join("m.yaml"), GOOD).unwrap();
        let events = rx
            .recv_timeout(Duration::from_secs(20))
            .expect("an event in 20 s")
            .unwrap();
        assert!(events.iter().any(|e| e.path.ends_with("m.yaml")));
    }

    /// Watch `file`, with `events` arriving and then stopping; the output
    /// and the error stream
    fn watch_events(
        file: &Path,
        validate_only: bool,
        verbose: bool,
        events: Vec<DebounceEventResult>,
    ) -> (ForgeResult<()>, String, String) {
        // Not `capture`: it reads the clear-screen code as a colour code
        let (mut out, mut err, mut input) = (Vec::new(), Vec::new(), std::io::empty());
        let result = watch_with(
            &mut Console::new(&mut out, &mut err, &mut input),
            file,
            validate_only,
            verbose,
            |_| {
                let (tx, rx) = channel();
                for event in events {
                    tx.send(event).unwrap();
                }
                Ok((rx, ()))
            },
        );
        (
            result,
            String::from_utf8(out).unwrap(),
            String::from_utf8(err).unwrap(),
        )
    }

    #[test]
    fn watching_runs_the_model_at_once_and_again_on_each_change() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("m.yaml");
        fs::write(&file, GOOD).unwrap();
        let canonical = file.canonicalize().unwrap();
        let changed = || Ok(vec![any(canonical.to_str().unwrap())]);
        let unrelated = Ok(vec![any("/elsewhere/notes.txt")]);
        let broken = Err(notify::Error::generic("disk gone"));

        let (result, out, err) =
            watch_events(&file, false, true, vec![changed(), unrelated, broken]);
        result.unwrap();
        for line in [
            "Forge - Watch Mode",
            "Mode: calculate",
            "Watching directory:",
            "Initial run...",
            "Calculation complete",
            "Change detected at",
            "2 scalars calculated",
            "\x1B[2J",
        ] {
            assert!(out.contains(line), "{line} in {out}");
        }
        assert_eq!(out.matches("Change detected at").count(), 1, "{out}");
        assert!(err.contains("Watch error: disk gone"), "{err}");
        assert!(err.contains("Channel error"), "{err}");

        let (result, out, _) = watch_events(&file, true, false, vec![changed()]);
        result.unwrap();
        assert!(out.contains("Mode: validate only"), "{out}");
        assert!(out.contains("Validation passed"), "{out}");
        assert!(
            !out.contains("\x1B[2J") && !out.contains("Watching directory"),
            "{out}"
        );
    }

    #[test]
    fn watching_says_when_the_model_fails() {
        let dir = tempfile::tempdir().unwrap();
        let stale = dir.path().join("stale.yaml");
        fs::write(&stale, GOOD.replace("value: 4", "value: 5")).unwrap();
        let (result, out, _) = watch_events(&stale, true, false, vec![]);
        result.unwrap();
        assert!(out.contains("Validation failed:"), "{out}");
        assert!(out.contains("1 value mismatches"), "{out}");

        let broken = dir.path().join("broken.yaml");
        fs::write(&broken, "a: [1, 2\n").unwrap();
        let (result, out, _) = watch_events(&broken, false, false, vec![]);
        result.unwrap();
        assert!(out.contains("Calculation failed:"), "{out}");
    }

    #[test]
    fn watching_a_file_that_is_missing_or_has_no_folder_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let (result, _, _) = watch_events(&dir.path().join("absent.yaml"), false, false, vec![]);
        assert!(result.unwrap_err().to_string().contains("File not found"));
        let (result, _) = capture(|c| watch(c, &dir.path().join("absent.yaml"), false, false));
        assert!(result.unwrap_err().to_string().contains("File not found"));
        let (result, _, _) = watch_events(Path::new("/"), false, false, vec![]);
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Cannot determine parent directory"));
    }

    #[test]
    fn the_time_is_hours_minutes_seconds_utc() {
        let time = chrono_lite_timestamp();
        assert_eq!(time.len(), 12, "{time}");
        assert!(time.ends_with(" UTC"), "{time}");
        assert_eq!(&time[2..3], ":");
        assert_eq!(&time[5..6], ":");
    }

    /// What watching `text` prints, with the events stopping at once
    fn watched(text: &str, validate_only: bool, verbose: bool) -> String {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("m.yaml");
        fs::write(&file, text).unwrap();
        let (result, out, _) = watch_events(&file, validate_only, verbose, vec![]);
        result.unwrap();
        out
    }

    #[test]
    fn watching_says_why_a_model_does_not_validate_or_calculate() {
        let unknown = "_forge_version: \"5.0.0\"\noutputs:\n  b:\n    value: null\n    formula: \"=nosuch + 1\"\n";
        for validate_only in [true, false] {
            let out = watched(unknown, validate_only, false);
            assert!(out.contains("failed:"), "{out}");
            assert!(out.contains("nosuch"), "{out}");
        }
        let random = "_forge_version: \"5.0.0\"\noutputs:\n  g:\n    value: null\n    formula: \"=MC.Normal(0.15, 0.05)\"\n";
        let out = watched(random, true, false);
        assert!(out.contains("forge simulate"), "{out}");
        let out = watched("a: [1, 2\n", true, false);
        assert!(out.contains("Validation failed:"), "{out}");
    }

    #[test]
    fn a_stored_value_that_is_missing_is_not_a_mismatch() {
        let missing = GOOD.replace("value: 4", "value: null");
        let out = watched(&missing, true, true);
        assert!(out.contains("Found 1 tables, 2 scalars"), "{out}");
        assert!(out.contains("Validation passed"), "{out}");
    }

    #[test]
    fn calculating_prints_what_verbose_asks_for() {
        let tables_only = "_forge_version: \"5.0.0\"\nt:\n  n: [1, 2]\n  d: \"=n * 2\"\n";
        let out = watched(tables_only, false, true);
        assert!(out.contains("t (2 columns)"), "{out}");
        assert!(!out.contains("scalars calculated"), "{out}");
        let out = watched(GOOD, false, false);
        assert!(out.contains("Calculation complete"), "{out}");
        assert!(
            !out.contains("Found") && !out.contains("scalars calculated"),
            "{out}"
        );
    }

    #[test]
    fn watching_stops_when_the_watcher_cannot_start() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("m.yaml");
        fs::write(&file, GOOD).unwrap();
        let (mut out, mut err, mut input) = (Vec::new(), Vec::new(), std::io::empty());
        let result = watch_with(
            &mut Console::new(&mut out, &mut err, &mut input),
            &file,
            false,
            false,
            |_| {
                Err::<(Receiver<DebounceEventResult>, ()), _>(ForgeError::Validation(
                    "no watcher".to_string(),
                ))
            },
        );
        assert!(result.unwrap_err().to_string().contains("no watcher"));
        assert!(!String::from_utf8(out).unwrap().contains("Initial run"));
    }
}
