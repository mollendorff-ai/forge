# Changelog

All notable changes to Forge will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed
- **`docs/EXAMPLES.md` showed invented outputs, and most of its models did not run.** The Monte Carlo, decision-tree, tornado and bootstrap sections printed placeholder numbers (`45678.91`, `245678.90`) as Forge's output and drew conclusions from them ("73% probability of positive NPV"; it is 100%). Every Monte Carlo model used settings that do not exist (`thresholds:` lists, `sensitivity: true`, `MC.Correlated`, distributions in table columns); the SaaS example used `COUNTIF(x, > 15000)` for `">15000"`; the Quick Start claimed `calculate` replaces table formulas with numbers; the decision tree counted the R&D cost twice and called a tie a win; the tornado model made revenue growth cancel out exactly while the text named it the top driver; and one example ran `forge monte-carlo`, which does not exist. All models now run, every output block is what Forge prints, the conclusions follow from it, and each section has a check by hand.
- **`monte_carlo` ignored `percentiles`**: an output always showed P5 to P95, so a requested P1 or P99 never appeared. The requested ones are now added. A percentile above 100, or a `threshold` Forge cannot read (it was silently skipped), is now an error.
- **Formulas in multi-document files could not refer to any table or scalar group** (ADR-052). Names were stored as `document.name`, but formulas were left as written, so `=SUM(sales.tax)` and even `=totals.tax * 2` in the same document were unknown. The documents of a file now share one namespace: a formula uses any document's tables and scalars by their usual names. A name defined in two documents is an error naming both, and a formula that starts with a document label says what to write instead.
- **`forge calculate` did not write results into multi-document files.** It now does, document by document, with the same in-place editing and checks as for one file.
- **`forge import --split-files` wrote files whose formulas could not work** when one part used another's table or scalars. It now refuses before writing anything and lists the formulas; a self-contained model is still split.
- **`forge import --split-files` and `--multi-doc` wrote files that did not validate** for any workbook with a table: they declared `_forge_version: "1.0.0"`, which allows no tables. They now declare 5.1.0. Formulas that refer across the split documents still do not resolve (each document is read under its own name); FEATURES.md says so and the roadmap has it.

### Added
- **`tests/doc_examples.rs`** runs every model in docs/EXAMPLES.md with the command its sections call for, and checks every number in the output block after it against the actual result at the precision shown (16 models, 109 numbers); it also checks the real-options and Bayesian tables against their example files. A planted wrong number fails it.

### Changed
- **Result names in multi-document models no longer start with the document's `_name`** (`totals.tax`, not `summary.totals.tax`); `_name` is a label. Dotted table names, which only came from that prefix, no longer arise.
- `forge import --multi-doc` of the bagel example calculates to exactly the numbers of the single-file import (tested).
- **`forge import` writes model format 5.1.0** (ADR-051): inputs as plain numbers, formulas as `{formula, value}` with no `value: null` or `formula: null`, and whole numbers as `100` instead of `100.0`. Every calculable example (including the bagel models) still round-trips through Excel to the same numbers.

## [10.0.0-rc.6] - 2026-11-08

Sixth release candidate. It adds model format 5.1.0 (ADR-051): inputs as plain numbers, formulas without `value: null`, and results written into the file next to their formulas without touching anything else. In rc.5, `forge calculate` deleted the comments of every file it wrote. No wrong answer was found since rc.5, but the stable release should be the candidate that carries this format, so its two weeks start from here (no earlier than 2026-11-22, the same day as before).

### Added
- **Model format 5.1.0: the short form (ADR-051).** An input can be a plain number (`price: 16`) and a formula scalar a mapping with only `formula:`, so a model no longer needs `value: null` or `formula: null` anywhere. The long form stays valid in 5.1.0 and 5.0.0. `forge upgrade` (now defaulting to 5.1.0) converts a 5.0.0 model in place, keeping its comments, and refuses to write if the converted file would not read as the same model. `forge schema v5.1` prints the new schema. All files in `examples/` are converted; every command gives byte-identical results on them before and after (58 outputs compared, plus the Excel export cell by cell). `test-data/` and the validation repositories stay at 5.0.0, so the long form keeps being tested.
- **ADR-051** records the decision: one file with formulas and their results, like Excel; format 5.1.0 because the change is backward compatible; `_forge_version` keeps its name and is documented as the model format version, independent of the tool version.

### Fixed
- **`forge calculate` damaged the file it wrote to.** It serialized the whole YAML again, which deleted every comment, removed quotes and turned `16` into `16.0`. It now writes only the `value:` of formula scalars, editing the text in place, and checks that nothing else changed before saving. It says how many values it updated, and does not rewrite the file (or make a `.bak`) when nothing changed. It also no longer overwrites a table column named `value` with numbers.
- **`forge upgrade` deleted comments** in the same way; converting to 5.1.0 now keeps them.
- **A group was read as a table or as scalars depending on the order of its keys.** A table whose first entry was a column with metadata (`y: {formula: ..., unit: ...}`) could be read as a group of scalars, dropping its data columns. Any list now makes it a table.
- **Decision-tree paths and the order of scenario errors changed between runs** (hash maps), against STABILITY.md's "same input, same output". They now follow the file.
- **Two examples in docs/EXAMPLES.md did not run** (a scenario model used names only its scenarios defined; a bootstrap model's formulas read the bootstrap settings), and their output blocks were invented numbers that drew the wrong conclusion. Both now run, and show real output with a hand check. Other invented output blocks remain and are on the roadmap (T1-EXAMPLES-OUTPUT).
- Clear errors for the likely mistakes: a plain number in a 5.0.0 file says to use 5.1.0; a bare formula string in a group of scalars says to write `{formula: ...}`; an unknown `_forge_version` says it is the model format, not the Forge version.

### Removed
- `writer::update_yaml_file`, `writer::update_scalars`, `cli::needs_schema_upgrade` and `cli::auto_upgrade_schema`. They rewrote whole files (losing comments), and nothing called them; nothing now changes a file's format on its own.

## [10.0.0-rc.5] - 2026-11-08

Fifth release candidate. `forge variance` in rc.4 called cost overruns favorable (a wrong answer; first entry under Fixed), so the two weeks before the stable release start again from this candidate. Every analysis command has now been checked once on a real model against an outside calculation (`examples/bagel/`), and all of them stay supported (ADR-050).

### Fixed
- **`forge variance` called cost overruns favorable (wrong answer).** Whether a difference was good was guessed from three words in the name (`cost`, `expense`, `cogs`), so rent, wages, fees and hours over budget were reported as favorable. Forge now uses a `better: lower | higher` field on the scalar, then a wider list of words that clearly mean a cost or a result, and otherwise says the direction is unknown (`is_favorable: null`, "No direction") instead of guessing. Values exactly on budget are "On target" and no longer counted as favorable.
- **`forge variance` percentages had the wrong sign against a negative budget.** A first-year NPV going from -95,580 to -123,259 showed as +29%. The percentage is now of the size of the budget, so it is -29%. `VARIANCE_PCT` in formulas changes the same way (see Changed).
- **`forge variance` compared a scalar missing from one file with 0.** It is now listed under `not_compared` instead.
- **`forge compare` and `forge real-options` printed and returned results in a different order on each run** (hash maps), against STABILITY.md's "same input, same output". Results are now in model order (compare) and by name (real options).
- **MCP `forge_bootstrap` returned every resampled statistic**: 20,000 numbers, about 74,000 tokens, in one reply. It now returns nine percentiles of the same distribution; `forge bootstrap --output` still writes all of them.
- **Decision trees broke exact ties silently**, by name. A decision node whose branches are worth the same now lists the others under `tied_with`, and the CLI says the choice is not made by the numbers.
- **The variance reports said "Generated by Forge v2.3.0".** They now give the real version.
- **The Bayesian example said a node can have only one parent.** Several work (`"rainy, yes": [0.9, 0.1]`); the example and EXAMPLES.md now say so.

### Changed
- **`VARIANCE_PCT(actual, budget)` divides by the size of the budget**, `(actual - budget) / ABS(budget)`, the usual FP&A convention: `VARIANCE_PCT(-120, -100)` is -20% (a bigger loss), not +20%. Results for a positive budget are unchanged.

### Added
- **ADR-050: all analysis engines stay supported** under the STABILITY.md promises; none is marked experimental. The architecture index also lists ADR-047 to ADR-049, which it had missed.
- **`better: higher | lower`** on a scalar, for `forge variance`. Accepted by both schemas.
- **`examples/bagel/` covers every analytics command**: `actual.yaml` (variance), `second_shop.yaml` (decision tree), `wait_option.yaml` (real options), `first_month.yaml` (bootstrap), `slow_week.yaml` (a two-parent Bayesian network). `tests/bagel_example.rs` asserts each headline number against an independent calculation shown in the test (roadmap T1-USE).

## [10.0.0-rc.4] - 2026-11-07

Fourth release candidate. rc.3 could report a scenario's results with a misspelled override silently ignored (a wrong answer; first entry under Fixed), so the two weeks before the stable release start again from this candidate. Also in it: goal seek converges properly, `forge scenarios` shows results, the roadmap is rewritten, and the bagel example with externally checked numbers.

### Added
- **`examples/bagel/`: a 12-month plan for a fictional bagel shop**, with an included costs file, four scenarios and a Monte Carlo version. Its headline numbers are asserted in `tests/bagel_example.rs` against a calculation written independently of Forge.
- **Scenarios can change a value from an included file**: `"@costs.prices.flour": 1.70`. The included file is calculated with the new value.

### Changed
- **CI runs for changes under `tests/` and `examples/`.** The path filter listed only `src/`, so a pull request that added or changed an integration test got no checks.
- **The roadmap (`.asimov/roadmap.yaml`) is rewritten around trust before reach**, with the current state at the top, an acceptance criterion on every item, and the unverified commands named; the README links to it and lists them. `.asimov/project.yaml` now describes the real stack (axum, rmcp, indexmap) and conventions the code follows.

### Fixed
- **A scenario override that named no scalar was ignored (wrong answer).** A misspelled key under a scenario's `scalars:` (`price_per_dozn` for `price_per_dozen`) created a new scalar that nothing read, so `forge scenarios`, `forge calculate --scenario` and MCP `forge_scenarios` reported the base numbers under the scenario's name. It is now an error that suggests the closest name. Tornado had the same bug and was fixed earlier; goal seek, break-even and sensitivity already reject unknown names.
- **Goal seek and break-even said `converged: false` on good answers.** One `tolerance` was used both for the width of the input range and for the error in the output, so a target in the thousands could never be "converged" within 0.0001. `tolerance` is now how close the output must get to the target, and the search continues until it does or the input cannot be split further. The CLI uses the same search as MCP (it had its own copy), and each step calculates the model once instead of twice.
- **`forge scenarios` showed the wrong things.** `--scenario` / MCP `scenario_filter` was ignored; each scenario listed the first five inputs alphabetically, rounded to two decimals (0.026 became 0.03), and not its results; `outputs`, `probability_positive` and `ranges` were always empty. Each scenario now shows its calculated scalars, and every calculated scalar gets an expected value, a range and a probability of being positive.
- **MCP `forge_calculate` on a model with `MC.*` distributions said `Unknown function: MC.PERT`.** It now gives the same message as the CLI: those scalars need `forge simulate`.

## [10.0.0-rc.3] - 2026-11-07

Third release candidate. No wrong answer was found in rc.2. This candidate adds features (include references, inline content in the REST API) and fixes crashes and wrong output order, so the stable release should be this candidate, and its two weeks start from here.

### Added
- **The REST API takes the model as text, and answers with the numbers.** `validate`, `calculate` and `audit` accept `content` (and `includes`) instead of `file_path`, like the MCP tools do, so a client on another machine no longer needs the file on the server. They now return the structured `result` (values, dependency tree, mismatches) instead of only a "completed" message, and `calculate` adds `calculated_content` for inline models and accepts `scenario`. Both servers share one inline-content path, including the check that include paths cannot leave the temporary directory. `file_path` is no longer required in these three requests; the response fields that were there are unchanged.
- **Formulas can read values from included files: `@namespace.name`.** `_includes` loaded other files, and the docs, the test data and `ParsedModel::resolve_namespace_ref` all described `@sources.pricing.unit_price`, but the calculator never resolved it: `@` was a syntax error, and without it the name was an unknown column. Now `=amount * @rates.fx.usd_eur` works in row and scalar formulas, with the CLI and with MCP inline `includes`. Each included file is calculated first, with its own includes, so a value it defines by formula is available. A name the file does not have is an error that names the `_includes` entry it needs, and a failing formula in an included file names the file. A group with the same name as a namespace does not create a false cycle. Only scalars can be read. `forge export` writes a formula that reads an included value as its calculated value and lists it. Documented in FEATURES.md (Values from Other Files) and MCP.md.

### Removed
- **The function registry's `demo` flag.** All 173 definitions said whether a function belonged to a "demo build" that does not exist. The field, `demo_functions`, `count_demo`, `demo_by_category` and `is_demo_function` are gone, `enterprise_functions` and `count_enterprise` became `all_functions` and `count_all`, and the "DEMO/ENTERPRISE" labels in the evaluator comments are removed.
- `ParsedModel::resolve_namespace_ref`, the old include lookup. It read values without calculating them and nothing called it; `@namespace.name` formulas replace it.

### Fixed
- **`forge functions | head` panicked.** A closed pipe made `println!` panic with a backtrace. The command-line commands now end quietly when the reader goes away, like other tools. The MCP and API servers are unchanged.
- **A NaN in the data (`.nan` in YAML) panicked `MEDIAN`, `MIN`, `MAX`, `SMALL`, `LARGE`, `PERCENTILE`, `QUARTILE`, `RANK`, `SORT`, Monte Carlo, bootstrap, Bayesian and decision-tree code.** All sorts and comparisons now use a total order, so the commands give an answer or a clear error instead of a crash.
- **`forge audit` split dotted names.** `summary.total` showed as two unknowns (`summary`, `total`), so the tree for any grouped scalar or table column was wrong. Dotted names, `table.column` and `@namespace.name` now stay whole and resolve to the right scalar or column.
- **The inline examples in `docs/MCP.md` did not run.** They used an older model format (`name:`, `tables:`, `columns:`), so the first call a new user copied failed with `Missing required field: _forge_version`. They now use the 5.0.0 format. A unit test (`test_mcp_doc_inline_examples_run`) runs every inline example in the guide against the server, so the guide and the server cannot drift apart again.
- **`forge export` wrote included files' sheets uncalculated.** A value an included file defines by formula was an empty cell on its sheet (`rates.Scalars`). The included model is now calculated before it is written.

### Changed
- **Tables, columns and scalars keep the order of the model file.** `forge calculate` printed them alphabetically, `forge export` wrote columns and scalar rows alphabetically (so `ebit` came before `revenue`), and `forge import` wrote data columns first, then formula columns. All three now follow the file: the README model prints and exports `year, revenue, gross_profit, opex, ebit, tax, net_income`. Numbers are unchanged; the order is still the same on every run. A data column and a formula column keep their places relative to each other, which needed a recorded order because the two live in separate maps (`Table::column_names`). On export, table sheets still come before scalar sheets. The MCP and JSON results are keyed by name as before; their `columns` list is now in model order. With two or more included files, their sheets could change order between exports (they came from a `HashMap`); they are now sorted by namespace.
- **`forge functions --json` and the MCP `forge_functions` tool no longer return `"edition": "enterprise"`.** There is no such edition. The field is removed; `total` and `categories` are unchanged. The REST API root and the `forge_schema` tool description no longer say "enterprise" either.
- **`docs/architecture/00-OVERVIEW.md`: unsourced figures removed.** The cost per year, CO₂ per formula, tokens per formula, "128 minutes for AI", "2133 tests", "640x faster", "zero bugs", "100% accuracy" and development-velocity numbers had no source in the repository. The overview now points to VALIDATION.md for what is measured, and a dated note says the architecture below predates the release candidates.
- **`forge schema` no longer calls v5 "Full enterprise support"**; it says "Full feature set". Comments that called modules or functions "Enterprise Only" or "only in full build" are corrected: there is one build, and it has everything. The registry's demo/enterprise labels are unchanged.
- **The unused `xlformula_engine` dependency is removed**, and 12 packages with it (among them `pest` and `stacker`). Forge evaluates formulas with its own evaluator; no source file used the crate. The overview no longer lists it.

## [10.0.0-rc.2] - 2026-11-06

Second release candidate. rc.1 could give a wrong answer when a table formula used a calculated scalar (first entry below), so the two weeks before the stable release start again from this candidate.

### Fixed
- **A table formula that used a calculated scalar read its stored value, not the calculated one (wrong answer).** Forge calculated every table before any scalar, so in a table formula such as `=x * assumptions.double`, where `assumptions.double` has a formula, it read whatever value was stored in the file (from an import, a hand edit or an earlier run) or failed with "Unknown column" when there was none. With `double` stored as 999 and calculating to 20, `y` came out as `[999, 1998, 2997]` next to `assumptions.double = 20`. Table columns and scalars are now calculated in one dependency order, cell by cell, and a formula's stored value is never read. A column can use a calculated scalar, a scalar can sum a calculated column, and `share = revenue / summary.total` with `total = SUM(t.revenue)` works without a false cycle. A real cycle through a column and a scalar is reported by name (`formulas t.y, s.k depend on each other`). The output of every example and of the README model is unchanged.
- **One failing formula hid the others.** `forge calculate` stopped at the first error, so a model with three mistakes took three runs to fix. It now calculates every formula that does not use a failed one and reports all failures together, each with its formula, followed by the formulas it could not calculate because they use a failed one (`calc.y (uses calc.x)`). A failed formula gets no value, so nothing can read a number from it; the command still exits 1 and writes nothing, and no values are printed. A single failure keeps the same message as before. The MCP `forge_calculate` tool returns the same list.
- **`forge calculate` showed no table values.** It printed `revenue (5 rows)` and nothing else, the MCP `forge_calculate` result had only column names and counts, and the file keeps formulas for columns, so the only way to see a projection was to export it. Calculate now prints every value of every column, and the MCP and JSON result has `values` for each table: plain arrays of numbers, text or booleans, at full precision. Numbers print at full precision, as scalars always have.
- **Calculation errors name the cell.** An error now says which column or scalar failed and shows its formula: `calc.x: Division by zero (formula: =assumptions.a / 0)` instead of `Eval: Eval error: Division by zero`. Errors in a table name the row, counted from 1 (`projections.ratio row 2: ...`). A circular dependency lists the names in the cycle (`scalars calc.a, calc.b depend on each other`, `calc.a refers to itself`, `columns t.p, t.q depend on each other`) instead of only saying there is one. The error kinds and exit codes are unchanged.
- **Stale text.** `forge functions --help` said Forge supports 58 functions (it has 173) and listed outdated category counts; the help now points to the generated list. The list header said "Forge Enterprise"; there is no such edition. README: removed a leftover validation table with older numbers. ADR-028 is marked not scheduled, since nothing in it is implemented. Links to the private, archived `forge-e2e` repository in seven ADRs now point to forge-e2e-gnumeric and forge-e2e-r.
- **`forge export` dropped `scenarios` without saying so.** Excel has no equivalent, so they are still not written, but the export now lists them as not exported (CLI and MCP `forge_export`), and the README lists it under known limitations. The CLI heading for these warnings is now "Not exported as Excel formulas".

## [10.0.0-rc.1] - 2026-11-06

First release candidate. [docs/STABILITY.md](docs/STABILITY.md) says what the stable release will promise; it follows after two weeks without a report of a wrong answer.

### Fixed
- **`PRICEDISC`, `YIELDDISC` and `ACCRINT` ignored the day-count `basis`** and counted actual days over 360; Excel's default is US 30/360. `ACCRINT` also ignored the coupon schedule. They now follow Excel's definitions for bases 0 to 4, and a basis outside 0 to 4 is #NUM!. Results with the default basis change for most dates.
- **`YEARFRAC`** now applies the US 30/360 end-of-February rules, Excel's actual/actual rule for spans over a year, and gives the same answer whichever date comes first.
- **`forge export`** now writes `IPMT`, `PPMT`, `EFFECT`, `NOMINAL`, `PRICEDISC`, `YIELDDISC`, `ACCRINT` and `REPT` as formulas; before, it wrote their calculated values and listed them as having no Excel equivalent.
- Checked in Excel for iPhone on 2026-11-06: 18 of 18 results match, including six where Gnumeric 1.12 differs (see VALIDATION.md).
- **`FV`, `PV`, `NPER`, `IPMT` and `PPMT` ignored payments at the start of the period.** They accepted Excel's `type` argument (1 = payment at the start of each period) and then computed as if it were 0, so `FV(0.04/12, 120, -200, -5000, 1)` gave 36,904.12 instead of 37,002.29. `type` = 0, the default, was right and is unchanged; `PMT` and `RATE` were right. All six now share one implementation, checked against numpy-financial on 456 combinations and against Gnumeric in forge-e2e-gnumeric (12 new cases).
- **A function with no result now returns #NUM! instead of NaN**, as in Excel (for example `NPER` when the payment never covers the interest).

### Added
- **Bootstrap interval methods**: `method: percentile` (default, as before), `basic` (reverse percentile) or `bca` (bias-corrected and accelerated, using a jackknife for the acceleration). The result and `-o` output name the method. BCa stops with a clear error when it is undefined for the data (all resamples on one side of the estimate, or an acceleration too large). Checked against scipy's `bootstrap` in the unit tests and against R `boot` in forge-e2e-r.
- **docs/STABILITY.md**: what a stable release will and will not promise, and what blocks the first one.
- **Bayesian networks with several parents per node.** A CPT row names one state of each parent, in the order of `parents`, separated by commas (`yes, off: [0.2, 0.8]`); spaces around the commas do not matter. Every combination of parent states needs exactly one row: a missing, duplicated or misnamed row is an error, never a silent probability of 0. Checked against hand calculations (rain-sprinkler-grass, including explaining away and swapped parent order) and against R `bnlearn` on the 3 multi-parent specs in forge-e2e-r. One-parent models are unchanged.
- **Validation workflow**: it now also runs on every pull request that changes `src/`, not only on the weekly schedule. On a pull request it uses the forge-e2e-gnumeric and forge-e2e-r branches with the same name as the Forge branch, when they exist, so a Forge change and its suite change are tested together before either is merged.

### Changed
- **Deterministic output order.** `calculate`, `scenarios`, `decision-tree` and `bayesian` now list tables, columns, scenarios and variables in sorted order, so the same input always gives the same output. Scenario weighted sums no longer differ in the last digits between runs.
- **`simulate`** prints its input variables, outputs and thresholds in name order too, so the same seed gives identical output on every run (apart from the execution time).
- **`forge validate` on Monte Carlo models** says plainly that validate cannot evaluate distributions and points to `forge simulate`, instead of an unrelated error.
- **`examples/quota_forecast.yaml`**: forecast values updated to the calculated ones; the example now validates (covered by a test).
- **YAML library**: `serde_yaml_ng` is replaced by `yaml_serde`, the YAML organization's maintained fork of `serde_yaml`. The old library depended on `unsafe-libyaml`, which is unmaintained (RUSTSEC-2024-0449, the one `cargo audit` warning); `yaml_serde` uses its maintained fork `libyaml-rs`. No code change was needed. Parsing, re-serialization and error messages were compared with the old library on all 99 YAML files in this repo and the two validation suites, plus 62 edge cases (YAML 1.1 booleans, octal and hex, infinities, anchors, merge keys, tags, duplicate keys, bad syntax): identical.
- **Dependencies**: `rmcp` 3 (the MCP server's `ServerInfo` is now `ServerConfig`; protocol output unchanged), `statrs` 0.19, `jsonschema` 0.60, `sha2` 0.11, `base64` 0.23.

## [10.0.0-beta.11] - 2026-11-05

### Changed
- **`forge real-options` result**: `total_option_value` is removed. It added option values that overlap (waiting and expanding depend on the same project), so it double counted. Each option now has a `project_value`, and the result has `best_option` and `project_value_with_options` (the best one). `traditional_npv` is optional and can be set in `real_options`; without it, options held on top of the project have no project value and the decision is `UNDECIDED`. `-o` now writes YAML instead of a Rust debug dump.

### Added
- **Dependabot** (`.github/dependabot.yml`): a weekly pull request for minor and patch updates of Cargo dependencies, one per major version bump, and one for GitHub Actions.
- **Validation workflow** (`.github/workflows/validation.yml`): builds Forge and runs the forge-e2e-gnumeric and forge-e2e-r suites every Monday and on demand. It also runs on pull requests that change its own file or `Cargo.toml` (releases and dependency updates), so the result shows on the PR.
- **Weekly CI** (`.github/workflows/weekly.yml`): `cargo audit` every Monday and on `Cargo.lock` changes, and tests plus strict clippy on the latest stable Rust every Monday.

### Fixed
- **`forge examples <name> --run`** looked for a program called `forge` on the PATH and failed with "No such file or directory" when Forge was run from a build folder or by full path. It now runs the same binary.
- **Arithmetic overflow** returned infinity (`2^1024`, `1E308*10`, `EXP(1000)`, `SUM(1E308, 1E308)` gave `inf`). Like Excel, it is now a `#NUM!` error, for operators and function results alike.
- **`forge calculate` on a Monte Carlo model** failed with `Unknown column: assumptions.revenue_growth` (on `examples/monte-carlo.yaml`), because it cannot evaluate `MC.*` distributions. It now says which scalars use them and to run `forge simulate`.
- **Error messages** were printed in Rust debug form (`Error: Validation("...")`). They are now plain text (`Error: Validation error: ...`); the exit code is still 1.

- **`--output` of `bootstrap`, `tornado`, `decision-tree`, `bayesian` and `scenarios`** wrote a Rust debug dump (`BootstrapResult { ... }`) that no tool could read, although the help says YAML. They now write YAML, or JSON for a `.json` path, like `simulate` and `real-options`. Found while building the forge-e2e-r runner for these commands.
- **`NETWORKDAYS` with a start date after the end date** returned 0. Like Excel, it now counts the same days and returns the result negated (`NETWORKDAYS(Mon 22 Jan, Mon 15 Jan)` is -6). Found by making the forge-e2e-r date validator call Forge instead of comparing hard-coded numbers.
- **Excel compatibility of `^` and boolean comparison**: `0^0` and a negative base with a fractional exponent are now `#NUM!` errors and `0^-1` is `#DIV/0!` (Excel, Gnumeric: same; Forge returned 1, NaN and infinity). `TRUE = 1` and `FALSE = 0` are now FALSE: in Excel and Gnumeric a boolean never equals a number (arithmetic still coerces, `TRUE + 1 = 2`). Models that relied on `IF(flag = 1, ...)` with a boolean `flag` must compare with `flag = TRUE` or use `IF(flag, ...)`.
- **`forge export`**: `^` is written without spaces (`2^10`, not `2 ^ 10`). Excel reads both; Gnumeric 1.12 could not read the spaced form and showed the formula as text.
- **`forge real-options`**: "Project Value" was the summed option values plus a traditional NPV that the CLI never set (always 0), so it equalled "Total Option Value". A defer option is now counted as the project value on its own (it already includes investing today); other options add to `traditional_npv`. The "Trigger" lines are labelled as rules of thumb, and options print in file order.
- **`forge export`**: formulas that referenced other sections came out as broken Excel. A scalar in a row formula became `'assumptions'!operating_margin2`, a column named like a function (`year`) was left as text, and in scalar formulas `projections.total_profit` and same-group names were not translated. Formulas are now translated token by token, with names resolved the way `forge calculate` resolves them: row formulas use same-row cells, scalars are absolute cells (`'assumptions'!$B$3`), and in scalar formulas (or where a function such as `MATCH` reads a whole column) a column is its full range. A table whose columns differ in length used the length of an arbitrary column for its ranges. A formula Excel cannot express with the same meaning is written as its value, with the Forge formula in a cell note and a warning. The model is calculated first, and each formula cell caches Forge's result. Checked by recalculating in LibreOffice: 1,600 of 1,656 formula cells from the forge-e2e-gnumeric models and all calculable cells of the examples agree. The rest are LibreOffice-versus-Excel differences, XLOOKUP (not in LibreOffice 24.2), and one model Forge cannot calculate.
- **`forge import` of exported workbooks**: `Name` / `Value` sheets now become scalar groups again, and references to other sheets come back as `table.column` and `group.name` (the old translator used the current sheet's column letters for every sheet). A short column is no longer padded with zeros. Formulas Forge cannot express are imported as values with a warning. Every forge-e2e-gnumeric model and every example that calculates comes back from Excel with the same numbers.
- **`forge tornado`**: bars were ranked by a made-up score (swing × relative range²) when the inputs were on very different scales, but drawn and printed with the real swing, so the order looked random. They are now ranked by the output swing that is shown, ties by name, so the same model always prints the same diagram. The number after the bar was labelled "±" but is the full low-to-high swing; it is now "swing", with the output at the low and high input beside it.
- **`forge import`**: the YAML it wrote was Forge's internal dump (`!Number` / `!Text` tags, no `_forge_version`), so `forge validate` rejected it. It now writes the same format as the examples, in single-file, `--split-files` and `--multi-doc` modes. Row formulas were also attached to the wrong columns (the formula grid was read as if it started at A1), and a reference to column `E` (`=C2/E2`) was left untranslated because `E` was treated as Euler's constant.

## [10.0.0-beta.10] - 2026-11-05

Version 10.0.0-beta.9 was tagged but never published (the release workflow did not start), so its changes are part of this release.

Fixes for wrong results that came back with no error, one security fix, and
documentation that now says what the project has and has not been checked
against. Found in an independent evaluation on 2026-11-04.

### Security
- **Dependencies**: all 21 `cargo audit` advisories are cleared (rustls, rustls-webpki, aws-lc-sys, h2, rmcp, quick-xml, zip). The Excel libraries moved to `calamine` 0.38 and `rust_xlsxwriter` 0.102.
- **MCP inline includes**: with inline `content`, an `_includes` path that was absolute or contained `..` could write the matching `includes` text to any file the server could write, or make the parser read a YAML file outside its temp directory. Only plain relative paths are accepted now. Earlier versions are affected, so update if you expose `forge mcp` to untrusted model content.
- **REST API (`forge serve`)**: CORS allowed every origin, so any website open in the browser could make a running `forge serve` read or write files, and any `Host` header was accepted (DNS rebinding). Browser requests from other origins are now refused unless listed with `--cors-origin`, a local server rejects foreign `Host` headers, `--token` / `FORGE_API_TOKEN` requires `Authorization: Bearer <token>` on `/api/v1/*`, and listening on a non-loopback address without a token is refused. Update if you run `forge serve`.
- **`forge update`**: downloads were installed without checking them. The archive's SHA-256 must now match the release's `checksums.txt`, downloads fail on an HTTP error instead of saving the error page, and a private temp directory is used. It also looked for asset names no release has, so it could not find an update on any platform; it now uses the published `forge-<target>` names and handles the Windows `.zip`.

### Fixed
- **Bayesian networks**: a node with two parents, which the inference does not support, gave wrong probabilities with no error. It is now rejected, and so is a CPT row that does not match a parent state. `examples/bayesian.yaml` used both and failed to load; it is now a one-parent chain with results checked against a hand enumeration, and README, `docs/FEATURES.md` and `docs/EXAMPLES.md` show the same working network with queries as `--query` / `--evidence` options.
- **Releases**: the release workflow now starts after the version tag is created. Tags pushed by CI used to leave it idle, so no binaries or crates.io package were published.
- **Array arithmetic**: an array in arithmetic, a comparison or a function that expects one number was turned into its length, so `=SUM(a * b)` gave 9 instead of 140. Operators now work element by element. Column lengths that do not match, or a column where one number is expected (`PMT`, `ROUND`, `TEXT` and others), are errors. `IF`, `AND`, `OR`, `XOR` and `NOT` accept arrays. A scalar formula that returns several values is an error.
- **IRR, XIRR, RATE**: these returned the last guess whether or not it had converged (`=IRR` of all-positive flows gave 5,778,119). They now find a rate or return an error. `IRR(values, guess)` no longer counts the guess as a cash flow, and `RATE` honours its `type` argument.
- **Schema**: valid models were rejected when a column formula started with `MAX(` or similar, or a column held dates or formulas. Five of the ten examples failed `forge validate`. Row formulas such as `=MAX(0, x)` now run row by row. `real_options`, `bootstrap` and `bayesian_network` sections are accepted.
- **Bare scalar names**: in `=revenue - cogs` inside `budget`, the name could resolve to a scalar of another section, and the result changed from run to run (`examples/variance.yaml`). Names now resolve to the formula's own section first. A name that matches several sections is an error.
- **Tornado**: short input names such as `growth_rate` now resolve to the unique matching scalar instead of producing empty bars. Ambiguous or missing names are errors. Thanks to @rafaeljara-cmyk.
- `examples/scenarios.yaml`: IRR now includes the acquisition cost, and the expected-output comment matches what the model calculates.
- `examples/breakeven.yaml`, `examples/bootstrap.yaml`: a text label stored as a scalar value is now a comment.
- **Monte Carlo `seed`, `sampling` and `correlations`**: a fixed seed gave different results on each run because variables were sampled in hash-map order; `sampling: latin_hypercube` was never used; `correlations` were never applied. Variables are now sampled in name order, uniform draws from the configured method go through each distribution's inverse CDF, and correlations are applied with a Gaussian copula.
- **Monte Carlo failed iterations**: an iteration whose formulas failed, or an output with no value, was recorded as 0. Both now stop the simulation with an error naming the iteration. `examples/monte-carlo.yaml` had every NPV at 0 for this reason; its IRR line (no up-front investment, so no answer) is removed.
- **Unknown keys**: analysis configs (`real_options`, `tornado`, `bootstrap`, `monte_carlo`, `decision_tree`, `bayesian_network`, scenario definitions) ignored keys they did not know, and scalars accepted any extra key. A misspelt key now fails with an error that names it. `examples/real-options.yaml` used `expansion_cost`, which was never read, so the Expand option was priced at $12.0M instead of $7.1M.
- **Decision-tree risk profile**: P(value > 0), best case and worst case used every terminal branch, including ones the optimal policy rejects, with local rather than path probabilities. `examples/decision-tree.yaml` showed 100%, $15M and -$2M instead of 42%, +$8M and -$5M. The risk profile now follows the optimal policy and lists its outcomes (`outcomes` in JSON/YAML output).
- `examples/real-options.yaml`, `examples/tornado.yaml`, `examples/bootstrap.yaml`, `examples/monte-carlo.yaml`, `examples/decision-tree.yaml`: expected-output comments now match results checked against independent calculations.

### Changed
- `forge serve --host <non-loopback>` now needs a token, and cross-origin browser apps need `--cors-origin`.
- A model or config with a key Forge does not recognise is now an error. Seeded Monte Carlo results differ from earlier versions, which were not reproducible.
- README, CLI help and crate description no longer claim validation or compliance that the repository does not show. A "Status and known limitations" section lists open problems.
- Clippy errors on Rust 1.97 fixed, so the Lint job passes.

## [10.0.0-beta.8] - 2026-03-14

### Changed
- **Dependency updates**: All direct dependencies brought to latest versions
  - `rmcp` 0.16 → 1.2 (MCP SDK major bump; adapted to non-exhaustive `ServerInfo`/`Implementation` builders)
  - `rust_xlsxwriter` 0.93 → 0.94, `calamine` 0.33 → 0.34, `jsonschema` 0.42 → 0.45
  - `clap` 4.5 → 4.6, `tokio` 1.49 → 1.50, `chrono` 0.4.43 → 0.4.44
  - `tempfile` 3.25 → 3.27, `anyhow` 1.0.101 → 1.0.102, `uuid` 1.21 → 1.22
  - `tracing-subscriber` 0.3.22 → 0.3.23, `assert_cmd` 2.1 → 2.2

## [10.0.0-beta.7] - 2026-03-10

### Added
- **Inline Excel export for sandboxed clients**: `forge_export` with `excel_path` omitted returns the workbook as a base64-encoded string in `excel_base64`, so Claude.ai, Cursor, and container-based MCP clients can receive Excel files without host filesystem access
- `ExcelExporter::export_to_buffer()` — in-memory workbook generation via `save_to_buffer()`
- `base64` v0.22 dependency

### Changed
- `ExportRequest.excel_path` is now optional (`Option<String>`). Existing callers that provide a path are unaffected.

## [10.0.0-beta.6] - 2026-03-10

### Fixed
- **Scenario overrides now reach grouped scalars**: Override keys like `growth_rate` automatically resolve to `assumptions.growth_rate` (or any other group) when no top-level scalar matches. Previously, overrides silently created an orphaned top-level variable while the grouped scalar stayed unchanged. Ambiguous names (same suffix in multiple groups) produce a clear error suggesting the full dotted path.

## [10.0.0-beta.5] - 2026-03-10

### Fixed
- **`forge_scenarios` and `forge_compare` now accept the documented `ScenarioDefinition` format**: Scenario parser detects structured format (`probability` + `description` + `scalars`) and extracts variable overrides from `scalars` only, skipping reserved keys. Previously, `description` (string) and `scalars` (nested object) triggered "must be a number" errors, making it impossible to use the documented format.
- `make install` now works as conventional alias for `make install-forge`

## [10.0.0-beta.4] - 2026-03-07

### Fixed
- **MCP tools work from sandboxed clients**: Tools no longer fail with "No such file or directory" when called from Claude.ai, Cursor, or container-based MCP clients that lack host filesystem access

### Added
- **Inline content mode**: All 16 file-path MCP tools accept `content` parameter as alternative to `file_path`, passing raw YAML directly without writing to disk
- **Inline includes**: New `includes` parameter (namespace → YAML map) resolves `_includes` entries without host filesystem access
- **Calculated content write-back**: `forge_calculate` with `content` + `dry_run: false` returns `calculated_content` field with computed YAML
- **Variance inline support**: `budget_content`/`actual_content` alternatives to `budget_path`/`actual_path`
- ADR-049: MCP Inline Content for Sandboxed Clients
- `tempfile` promoted from dev-dependency to runtime dependency

### Unchanged
- All 20 MCP tools maintain identical names and behavior for `file_path` callers
- `*_core()` function layer (ADR-047) is completely unchanged
- Parser, calculation engine, and CLI are unmodified
- `forge_import` remains file-based only (Excel I/O)

## [10.0.0-beta.3] - 2026-02-22

### Changed
- **MCP server migrated to rmcp** (Anthropic's official Rust MCP SDK)
  - Replaced 1,115 lines of hand-rolled JSON-RPC with rmcp derive macros
  - 20 tool definitions now use `#[tool]` attribute with typed request structs
  - Automatic JSON schema generation via `schemars` (replaces manual `json!({})` construction)
  - Server entry point is now async (tokio runtime)
  - ~722 net lines of code eliminated

### Added
- `src/mcp/types.rs` -- 20 typed request structs with automatic schema generation
- ADR-048: rmcp Migration decision record

### Unchanged
- All 20 MCP tools maintain identical names, parameters, and behavior
- `*_core()` function layer (ADR-047) is completely unchanged
- `forge mcp` CLI command works the same way
- CI MCP smoke test validates 20 tools

## [10.0.0-beta.2] - 2026-02-22

### Added
- **MCP integration guide** (`docs/MCP.md`): Setup for Claude Desktop/Code, 20-tool reference, workflows
- **Function reference** (`docs/FUNCTIONS.md`): Auto-generated reference for all 173 functions
- **MCP smoke test in CI**: Validates JSON-RPC protocol compliance and 20-tool count
- **Coverage CI job**: `cargo-llvm-cov` with dynamic badge (enforces ADR-004)
- **Dynamic badges**: Test count, coverage, and function count auto-updated by CI
- **Auto-generate FUNCTIONS.md**: CI commits updated reference on every push to main

### Fixed
- **CI clippy `use_self`**: Replace recursive type names with `Self` in enum/struct definitions
- **CI coverage build**: Remove `#[cfg(not(coverage))]` from `Path` import so `*_core()` functions compile under `cargo-llvm-cov`

### Changed
- **README MCP section**: Updated from 10 to 20 tools, removed outdated "CLI-only" note
- **README badges**: Static → dynamic gist-based shields.io endpoints

## [10.0.0-beta.1] - 2026-02-22

### Fixed

- **MCP stdout pollution**: Extracted `*_core()` functions from all CLI commands to eliminate `println!()` in JSON-RPC transport. MCP server now calls core functions that return structured result types, keeping the stdin/stdout protocol clean. (ADR-047)

### Added

- **7 MCP analysis tools**: `forge_simulate`, `forge_scenarios`, `forge_decision_tree`, `forge_real_options`, `forge_tornado`, `forge_bootstrap`, `forge_bayesian` — all 7 prediction engines now accessible to AI agents via MCP
- **3 MCP discovery tools**: `forge_schema`, `forge_functions`, `forge_examples` — AI agents can now query available functions, JSON schemas, and runnable examples
- **ADR-047**: Dual CLI/MCP Output Architecture documenting the core extraction pattern

### Changed

- **MCP tools return structured JSON**: All 20 MCP tools now return typed, serializable result structs instead of formatted text, enabling programmatic use by AI agents
- **MCP tool count**: 10 tools → 20 tools (10 existing + 7 analysis + 3 discovery)

## [10.0.0-alpha.12] - 2026-02-16

### Open Source Release

- **License**: MIT OR Apache-2.0 (standard Rust dual license)
- **Documentation**: Updated ADRs, README, CONTRIBUTING, editor extensions
- **Codebase cleanup**: Simplified project.yaml, roadmap, and source comments

### Code Quality (FOSS-004)

- **Fixed 1566 clippy pedantic/nursery/cargo warnings** across 180+ files — zero warnings remain
- Moved lint configuration from 20 Makefile `-A` flags to `Cargo.toml` `[lints.clippy]` section
- Pre-commit hook now enforces pedantic linting via `make lint`
- Added `# Errors`, `# Panics` doc sections to all public functions
- Added `#[must_use]` to pure functions throughout the codebase
- Replaced `push_str(&format!(...))` with `write!`/`writeln!` for string building
- Replaced `as f64` with `f64::from()` where lossless conversion exists
- Added underscore separators to numeric literals for readability
- Used `mul_add()` for fused multiply-add operations
- Documented all `#[allow(clippy::...)]` exceptions with invariant rationale

## [10.0.0-alpha.11] - 2026-02-16

### Changed

- **BREAKING**: Consolidated 3 binaries into single `forge` CLI
  - `forge-mcp` → `forge mcp` subcommand
  - `forge-server` → `forge serve` subcommand (with `--host`, `--port` args)
  - Simpler installation, smaller disk footprint
- Removed `(enterprise only)` from all `--help` text (13 occurrences)
- Updated MCP config: `"command": "forge", "args": ["mcp"]`
- Updated CI/CD workflows for single binary (UPX, archives, checks)
- Updated documentation: FEATURES.md, ADRs (005, 006, 012, 08), CHANGELOG_ARCHIVE

## [10.0.0-alpha.10] - 2026-02-16

### Security

- **bytes** 1.11.0 → 1.11.1: Fix integer overflow in `BytesMut::reserve`

### Dependencies (Breaking)

- **jsonschema** 0.38 → 0.42: Updated `validator_for` API
- **calamine** 0.32 → 0.33
- **rand** 0.9 → 0.10: Migrated to `RngExt` trait, updated RNG construction
- **rand_distr** 0.5 → 0.6
- **rust_xlsxwriter** 0.92 → 0.93

### Dependencies (Compatible)

- 87 packages updated via `cargo update` (anyhow, clap, chrono, tokio, etc.)

## [10.0.0-alpha.9] - 2026-02-16

### Changed

- **Makefile**: Removed legacy `install-system`, `install-user`, `install` targets
- **Makefile**: `uninstall` now only handles `~/.cargo/bin`
- **project.yaml**: Updated `install_path` references to `~/.cargo/bin`
- Standardized all install paths on `~/.cargo/bin` (single location)

## [10.0.0-alpha.8] - 2026-01-24

### Changed

- **Install location**: `make install-forge` now installs to `~/.cargo/bin/` instead of `~/bin/`
  - `~/.cargo/bin` is the default Rust bin directory, automatically in PATH for Rust users
  - No need to manually add directory to PATH

## [10.0.0-alpha.7] - 2026-01-24

### Codebase Cleanup

Major cleanup removing orphaned files, consolidating documentation, and fixing organizational issues.

### Deleted (29 files)

- **Documentation**: `docs/INSTALLATION.md`, `docs/VERSION_HISTORY.md`, `FP&A_TEST_COVERAGE_AUDIT.md`
- **ADRs**: `ADR-003-EDITOR-EXTENSIONS.md` (superseded by ADR-005)
- **Schemas**: `forge-v1.0.schema.json.deprecated`, `.asimov/protocols.json`
- **Scripts**: `bin/release.sh`, `bin/session-end.sh`, `references.yaml`
- **Examples**: 4 orphaned .rs debug scripts
- **Test data**: 6 redundant test files, 11 files consolidated into 2

### Renamed/Moved (14 files)

- `ADR-004-XLFORMULA-EQUIVALENCE.md` → `ADR-011-XLFORMULA-EQUIVALENCE.md` (fix duplicate number)
- `ADR-024-SELF-UPDATE-REMOVAL.md` → `ADR-024-SELF-UPDATE.md` (fix misleading title)
- Design docs converted to ADRs: `ADR-033-EXCEL-EXPORT`, `ADR-034-EXCEL-IMPORT`, `ADR-035-RICH-METADATA`
- `test-data/v1.0/` → `test-data/examples/` (contents are v5.0.0, not v1.0.0)
- `test-data/v4_*.yaml` → `test-data/advanced/`
- `test-data/quota_forecast.yaml` → `examples/`

### Updated (14 files)

- Fixed ADR cross-references (ADR-012, ADR-015, ADR-030)
- Updated `00-OVERVIEW.md` stats (60→173 functions, 183→2133 tests)
- Fixed `.markdownlint.json` duplicate keys
- Updated `schema/README.md` to document both v1.0.0 and v5.0.0 schemas
- Removed unused `async-trait` dependency
- Fixed clippy warnings in `schema.rs` and `update.rs`

### Consolidated

- 8 function test files → `test_all_functions.yaml`
- 3 IF test files → `test_if_function.yaml`
- Archived CHANGELOG v7.x and earlier → `docs/CHANGELOG_ARCHIVE.md` (2567→662 lines)

## [10.0.0-alpha.6] - 2026-01-24

### Rebranding: RoyalBit to Mollendorff AI

Complete organizational rebrand to disassociate from cryptocurrency scammers who hijacked the RoyalBit name.

### Changed

- **Crate renamed**: `royalbit-forge` → `mollendorff-forge`
- **GitHub repository**: `royalbit/forge` → `mollendorff-ai/forge`
- **Website**: `royalbit.ca/forge` → `mollendorff.ai/forge`
- **Author email**: `admin@royalbit.ca` → `admin@mollendorff.ai`
- **All source files**: Updated `use royalbit_forge` → `use mollendorff_forge`
- **Schema $id URLs**: Updated to `mollendorff-ai/forge`
- **Documentation**: All ADRs, README, FEATURES.md updated
- **Editor extensions**: Zed and VSCode extensions updated
- **Config files**: .cargo/config.toml, rustfmt.toml, audit.toml
- **.asimov files**: project.yaml, warmup.json updated

### Why Rebrand?

The "RoyalBit" name (company founded 2006) was hijacked by unrelated cryptocurrency scammers:

- **UK FCA Warning (Oct 2024)**: Official warning about "Royalbit Miners" - unauthorized firm
- **Fraudulent domains**: royalbit.ltd (trust score 38/100), royalbit.top, royal-bit.club
- **HYIP Ponzi schemes**: Offering impossible returns (155-580% in days)
- **Sources**: [FCA Warning](https://www.fca.org.uk/news/warnings/royalbit-miners), [Scam Detector](https://www.scam-detector.com/validator/royalbit-ltd-review/)

## [10.0.0-alpha.5] - 2026-01-03

### Changed

- **Self-update command restored** per ADR-024
  - `forge update` now functional against GitHub releases
  - Valid now that forge is public on GitHub

## [10.0.0-alpha.4] - 2026-01-02

### CLI Documentation: schema + examples commands

Two new CLI commands for improved discoverability and self-documentation.

### Added

- **`forge schema` command** - Display JSON schemas for model validation
  - `forge schema --list` - List available versions (v1.0.0, v5.0.0)
  - `forge schema v1` - Output v1.0.0 schema (scalar-only models)
  - `forge schema v5` - Output v5.0.0 schema (arrays, tables, advanced features)
  - Pipeable: `forge schema v5 > schema.json` for IDE integration

- **`forge examples` command** - Show runnable YAML examples
  - `forge examples` - List all 9 available examples
  - `forge examples <name>` - Display example YAML with comments
  - `forge examples <name> --run` - Display and execute example
  - `forge examples --json` - Machine-readable list for tooling

- **9 Example YAML Files** (`examples/` directory)
  - `monte-carlo.yaml` - Probabilistic simulation with distributions
  - `scenarios.yaml` - Probability-weighted scenario analysis
  - `decision-tree.yaml` - Sequential decisions with backward induction
  - `real-options.yaml` - Option pricing for managerial flexibility
  - `tornado.yaml` - One-at-a-time sensitivity analysis
  - `bootstrap.yaml` - Non-parametric confidence intervals
  - `bayesian.yaml` - Probabilistic graphical models
  - `variance.yaml` - Budget vs actual with VARIANCE functions
  - `breakeven.yaml` - Break-even with BREAKEVEN functions

- **ADR-032** - Documents schema/examples command design decisions

### Technical

- Schemas and examples embedded at compile-time via `include_str!()`
- Zero network dependency, works offline
- Examples guaranteed to match binary version
- 7 new unit tests for schema/examples commands

## [10.0.0-alpha.3] - 2025-12-30

### CI/CD, crates.io, and Multi-Arch Releases

This release establishes the full CI/CD pipeline and multi-platform distribution.

### Added

- **GitHub Actions CI** - Automated test, lint, build on every push
- **GitHub Actions Release** - Multi-arch builds triggered on version tags
- **crates.io publishing** - `mollendorff-forge` published automatically on release
- **README badges** - CI status, crates.io version, license badge
- **forge-e2e CI/CD** - Same pipeline for E2E validation tool

### Release Artifacts (5 platforms)

- `forge-x86_64-unknown-linux-musl.tar.gz` - Linux x86_64 (static musl)
- `forge-aarch64-unknown-linux-musl.tar.gz` - Linux ARM64 (static musl)
- `forge-x86_64-apple-darwin.tar.gz` - macOS Intel
- `forge-aarch64-apple-darwin.tar.gz` - macOS ARM (Apple Silicon)
- `forge-x86_64-pc-windows-msvc.zip` - Windows x86_64

### Fixed

- **proc-macro build failure** - Removed `+crt-static` from GNU targets in `.cargo/config.toml`
  - CI tests run on GNU/glibc (dynamic linking for proc-macros)
  - Release builds use musl targets (fully static)
- **forge-lsp archive error** - Removed non-existent binary from release workflow

### Binary Contents

Each release archive contains:
- `forge` - Main CLI tool
- `forge mcp` - MCP server for Claude Desktop/IDEs (was `forge-mcp`)
- `forge serve` - REST API server (was `forge-server`)

## [10.0.0-alpha.2] - 2025-12-29

### Documentation and README Overhaul

- **README.md** - Complete rewrite highlighting technical capabilities
- **FEATURES.md** - Updated function labels and descriptions
- **MARKET_ANALYSIS.md** - Updated competitive analysis
- **ADR-031** - License architecture decision record
- **Editor extensions** - Updated VSCode and Zed README files

## [10.0.0-alpha.1] - 2025-12-29

### Documentation Overhaul for Public Release

- **README.md** - Rewritten for open source release
- **FEATURES.md** - Updated function descriptions
- **VERSION_HISTORY.md** - Updated with accurate stats

## [10.0.0-alpha.0] - 2025-12-29

### BREAKING: Unified Single Binary

Forge is now a single binary with all 173 functions. Previous feature-gated builds consolidated.

### Removed

- Feature-gated build variants and Cargo.toml feature flags
- ~330 cfg attributes for conditional compilation
- Conditional stub functions in statistical.rs, financial.rs
- Conditional Makefile build targets

### Changed

- Single binary: `forge` (6.3MB) with all 173 functions
- Simplified CLI help
- Updated version to 10.0.0-alpha.0

### Stats

- 1297 tests passing
- 47 files changed, 84 insertions, 659 deletions
- Zero feature gates in codebase

## [9.9.6] - 2025-12-29

### Fixed

- **FORGE-011**: ISERROR now detects NA() as error value
  - Location: `src/core/array_calculator/evaluator/info.rs:18-28`
  - IFERROR now catches NA values like Excel

### Verified

- **FORGE-010**: INVALID - Could not reproduce, arrays load correctly

## [9.9.5] - 2025-12-28

### Changed

- **Test consolidation complete**: All unit tests moved inline per Rust idiom
  - Eliminated `src/core/array_calculator/tests/` directory (40+ files)
  - Eliminated `src/cli/commands/tests/` directory (20+ files)
  - Tests now inline in evaluator files using `#[cfg(test)] mod tests`
  - Integration/E2E tests remain in `../forge-e2e/`

### Evaluators Updated

- `evaluator/math.rs`: math, math_edge_cases, numeric_edge_cases
- `evaluator/trig.rs`: trig tests
- `evaluator/text.rs`: text, text_edge_cases, string_edge_cases
- `evaluator/logical.rs`: logical, logical_edge_cases, comparison_edge_cases
- `evaluator/advanced.rs`: advanced, advanced_function_edge_cases
- `evaluator/aggregation.rs`: aggregation tests
- `evaluator/array.rs`: array, array_function_edge_cases
- `evaluator/conditional.rs`: conditional, conditional_function_edge_cases
- `evaluator/forge.rs`: forge tests
- `evaluator/dates.rs`: date tests
- `evaluator/lookup.rs`: lookup tests

### Test Results

- Unit tests: 1496 passed (consolidated from 2006 - duplicates removed)
- Test coverage: 100% (173/173 functions)
- Zero warnings
- Zero tests/ directories in src/

## [9.9.4] - 2025-12-28

### Added

- **HLOOKUP tests**: 9 comprehensive tests for horizontal lookup function
  - `test_hlookup_exact_match` - exact match with FALSE range_lookup
  - `test_hlookup_range_match` - approximate match finding largest value <= lookup
  - `test_hlookup_default_range_lookup` - default TRUE behavior
  - `test_hlookup_value_not_found` - error handling
  - `test_hlookup_text_exact_match` - text value lookups
  - `test_hlookup_first_element`, `test_hlookup_last_element` - boundary tests
  - `test_hlookup_single_element` - single element array
  - `test_hlookup_range_lookup_boundary` - exact boundary values
  - File: `src/core/array_calculator/tests/lookup/lookup_advanced.rs`

### Verified

- **Monte Carlo tests**: 57 tests confirmed in `src/monte_carlo/*.rs` (inline pattern)
- **Information function tests**: 14 tests confirmed in `src/core/array_calculator/evaluator/info.rs`

### Test Results

- Unit tests: 1965 passed (was 1956, +9 HLOOKUP)
- Test coverage: 100% (173/173 functions)
- All gaps closed

### Notes

- Swarm analysis initially reported 16 untested functions
- Investigation revealed MC and Information functions were already tested inline (Rust pattern)
- Only HLOOKUP was actually missing dedicated tests

## [9.9.3] - 2025-12-21

### Fixed

- **FORGE-003**: DATEDIF Y and YD calculation errors (CRITICAL, 30 tests)
  - Y unit: Now counts only complete years (checks anniversary date)
  - YD unit: Calculates days since last anniversary correctly
  - MD unit: Uses actual days in previous month, not fixed 30
  - YM unit: Adjusts for incomplete months

- **FORGE-004**: COLUMNS(table) returns 1 instead of column count (7 tests)
  - COLUMNS now checks if argument is a table reference and returns column count
  - File: `src/core/array_calculator/evaluator/lookup.rs`

- **FORGE-005**: Table validation rejects valid test data (10 tests)
  - Deferred column length validation to calculation time
  - Only validates when row formulas are present (row-wise operations)
  - Tables with independent columns (used as separate arrays) now load correctly

- **TRIM function**: Now collapses multiple internal spaces to single space (Excel behavior)
  - Was only removing leading/trailing spaces
  - File: `src/core/array_calculator/evaluator/text.rs`

### Test Results

- E2E tests: 602 passed, 40 failed (was: 569 passed, 49 failed)
- Fixed 33 test cases
- Remaining 40 failures are forge-e2e test file issues:
  - Test files using undefined functions (D, GCD, FACT)
  - Test files with wrong argument counts (XIRR, PERCENTILE, COUNTUNIQUE)
  - Features not supported by `forge calculate` (scenarios, MC functions)
  - Expected behavior differences (floating point epsilon comparison)

### Notes

- FORGE-001 (Boolean equality with numbers) was already working correctly
- Unit tests: All 1956 tests pass

## [9.9.2] - 2025-12-17

### Added

- **FORGE-EXPORT-001**: Excel export for v5.0.0 scalar models (CRITICAL)
  - Scalar groups now export to separate worksheets (e.g., `utilities`, `scenario_probs`, `analysis`)
  - Each scalar group becomes its own worksheet with Name/Value columns
  - Cross-group formula references translated to Excel syntax: `scenario_probs.p_aligned` → `'scenario_probs'!B2`
  - Monte Carlo formulas (`MC.*`) export as calculated values with original formula in cell comment
  - Scalars without dots (e.g., `tax_rate`) go to default "Scalars" worksheet

### Changed

- `ExcelExporter::export_scalars()` now creates grouped worksheets instead of single "Scalars" sheet
- Added `ScalarLocation` struct to track worksheet and row for each scalar
- Added `translate_grouped_scalar_formula()` for cross-worksheet references

### Impact

- Unblocked DANEEL publication - all game theory models can now export to Excel
- Models affected: game-theory-asi-race.yaml, game-theory-asi-race-mc.yaml, game-theory-asi-bridge.yaml, alignment-bayesian-network.yaml, asi-decision-tree.yaml, asimov-real-options.yaml

## [9.9.1] - 2025-12-17

### Fixed

- **FORGE-MC-001**: Monte Carlo dependent formula evaluation broken (CRITICAL)
  - `forge simulate` sampled MC.* functions correctly but didn't propagate values through dependent formulas
  - All downstream calculations returned 0.0 instead of computed values
  - Fix: Changed simulate command to use `run_with_evaluator()` instead of `run()`
  - Each iteration now: substitutes sampled values → runs ArrayCalculator → extracts computed outputs
  - Added regression test

### Impact

- Unblocked DANEEL paper probabilistic analysis
- All Monte Carlo models with dependent calculations now work correctly

## [9.8.0] - 2025-12-17

### Added

- **ADR-027**: E2E Test Migration Strategy
- **MIGRATION_INVENTORY.md**: Complete test file inventory

### Changed

- **E2E Test Migration to forge-e2e**: Completed full migration using 8 parallel agents
  - Phase 1 (4 agents): Documentation & ADRs
  - Phase 2 (4 agents): Test file migration
  - Phase 3: Cleanup & validation

### Migration Summary

| Category | Files | Tests | Destination |
|----------|-------|-------|-------------|
| Roundtrip | 10 | 59 | forge-e2e/tests/e2e/roundtrip/ |
| Edge Cases | 9 | 143 | forge-e2e/tests/e2e/edge/ |
| E2E CLI | 7 | 70 | forge-e2e/tests/ |
| YAML Data | 34 | - | forge-e2e/tests/e2e/ |

### forge-e2e ADRs Created

- ADR-001: Testing Philosophy (unit vs e2e vs roundtrip)
- ADR-002: External Validation Engines (Gnumeric, R, Python)
- ADR-003: Edge Case Discovery Process
- ADR-004: Statistical Function Validation

### Result

- forge unit tests: 1,643 passing
- forge-e2e: 272+ tests migrated
- Context window now manageable for Claude

## [9.7.0] - 2025-12-17

### Verified

- **BUG-010 to BUG-013**: Confirmed already fixed by v9.6.0 changes
  - BUG-010: Boolean-number comparison in IF (`=IF(TRUE = 1, 1, 0)`) - works
  - BUG-011: String literal comparison in IF (`=IF("ABC" = "abc", 1, 0)`) - works
  - BUG-012: TRIM internal spaces consistent (`=LEN(TRIM("  a  b  "))` = 4) - works
  - BUG-013: `0^0` returns 1 (Excel convention) - works

### Added

- ADR-026: FPGA/HFT Acceleration (deferred post-capitalization)
- Performance optimization milestones to roadmap (SIMD, Rayon, GPU, Algorithmic)

### Changed

- Cleaned up roadmap: removed completed items, all history in CHANGELOG.md
- All 13 bugs (BUG-001 through BUG-013) now resolved

## [9.6.1] - 2025-12-17

### Fixed

- **Release asset naming**: Fixed Makefile to use expected naming convention
  - Binaries now correctly renamed before GitHub release upload

## [9.6.0] - 2025-12-17

### BUG-003 to BUG-009 Fixes - Edge Cases

Fixed multiple bugs discovered by E2E testing.

### Fixed

- **BUG-003**: TRUE/FALSE boolean literals now recognized in formulas
- **BUG-004**: Boolean vs Number comparison works (TRUE = 1, FALSE = 0)
- **BUG-005**: DATE rollover for invalid day params (e.g., Feb 29 in non-leap year → Mar 1)
- **BUG-006**: Verified ISERROR already implemented (no changes needed)
- **BUG-007**: EOMONTH function now available
- **BUG-008**: & concatenation operator added to tokenizer and evaluator
- **BUG-009**: DATE() now returns Excel serial number (not text string)

### Added

- `Token::Boolean(bool)` to tokenizer for TRUE/FALSE literals
- `Expr::Boolean(bool)` to parser AST
- `&` operator support for string concatenation
- Day overflow/underflow handling in DATE function
- Edge case test YAMLs

### Changed

- DATE() returns `Value::Number` (Excel serial) instead of `Value::Text`
- `values_equal()` handles Boolean vs Number comparisons
- String comparison remains case-insensitive (correct Excel behavior)

## [9.5.0] - 2025-12-17

### BUG-002 Fix - DATE Subtraction Now Works

Fixed date subtraction returning "Left operand must be a number" error.

### Fixed

- **DATE subtraction**: `=DATE(2024,12,31) - DATE(2024,1,1)` now returns `365`
- All date arithmetic operations now work correctly

### Changed

- `Value::as_number()`: Now coerces date strings (YYYY-MM-DD) to Excel serial numbers
- Added tests for date subtraction (leap year, non-leap year scenarios)

### Root Cause

The `as_number()` method didn't recognize date strings. DATE() returns "2024-06-15" (text) but binary operators require numbers. Fixed by adding date string → Excel serial coercion in `as_number()`.

## [9.4.0] - 2025-12-16

### BUG-001 Fix - Calculate Engine Missing Functions

Fixed mismatched feature gates that caused some functions to be listed but not evaluable.

### Fixed

- **Trig functions**: SIN, COS, TAN, ASIN, ACOS, ATAN now work in calculate
- **Math functions**: EXP, LN, LOG10, INT, SIGN, TRUNC, PI now work in calculate

### Changed

- `evaluator/math.rs`: Removed incorrect feature gates from math functions
- `evaluator/mod.rs`: Trig module always included
- `evaluator/trig.rs`: Corrected function availability

### Root Cause

Function registry listed functions as available but evaluator had mismatched feature gates, causing functions to appear in `forge functions` but fail in `forge calculate`.

## [9.3.0] - 2025-12-16

### Build Simplification - Full-Featured Default

Changed build system so default build includes all features.

### Changed

- **Default build is full-featured** - `cargo build` produces complete binary
- **Tests run everything by default** - `cargo test` covers all ~2700 tests
- Updated ~489 cfg attributes across the codebase
- Updated Makefile build targets
- Updated Cargo.toml binary definitions

### Added

- **ADR-025**: Build simplification architecture decision record

### Rationale

- Improves developer experience (no feature flags needed)
- Ensures complete test coverage by default
- Simplifies CI/CD pipeline

## [9.2.0] - 2025-12-16

### Self-Update Command Removal - Dead Code Cleanup

Removed the non-functional `forge update` command.

### Removed

- `src/update.rs` (28KB) - Update checking and binary replacement logic
- `tests/update_tests.rs` (4KB) - Unit tests for update module
- `Update` command from CLI
- 3 integration tests for update command

### Changed

- Updated README.md - removed update from commands list
- Updated docs/cli/README.md - removed update section

### Added

- **ADR-024**: Self-Update Removal architecture decision record

### Rationale

The `forge update` command pointed to a non-existent release endpoint. ~32KB of dead code removed. (Restored in v10.0.0-alpha.5 after GitHub releases were set up.)

## [9.1.0] - 2025-12-15

### CLI Commands for Prediction Methods

Added CLI commands for all prediction analysis methods.

### Added

- `scenarios` command - probability-weighted scenario analysis
- `decision-tree` command - backward induction solver
- `real-options` command - Black-Scholes and Binomial Tree pricing
- `tornado` command - sensitivity visualization
- `bootstrap` command - non-parametric confidence intervals
- `bayesian` command - Bayesian network inference

### Changed

- Updated --help header with current stats (2703 tests)
- Updated documentation with Monte Carlo and prediction methods

## [9.0.0] - 2025-12-15

### Bayesian Networks - Causal Probabilistic Modeling

Module for probabilistic graphical models with belief propagation.

### Added

- **Bayesian Networks module** (`src/bayesian/`)
  - Directed acyclic graphs (DAG) with conditional probability tables
  - Variable elimination inference (exact inference)
  - Evidence propagation and belief updating
  - pgmpy-validated calculations
- **ADR-023**: Bayesian Networks architecture decision record

### Features

- Discrete nodes with finite states
- Conditional probability tables (CPTs)
- Forward and backward reasoning
- Most likely explanation (MPE)
- YAML/JSON export of results

## [8.7.0] - 2025-12-15

### Bootstrap Resampling - Non-Parametric Confidence Intervals

Module for statistical inference without distribution assumptions.

### Added

- **Bootstrap module** (`src/bootstrap/`)
  - Resampling with replacement
  - Percentile confidence intervals
  - BCa (bias-corrected accelerated) method
  - Multiple statistics: mean, median, std, var, percentile
- **ADR-022**: Bootstrap Resampling architecture decision record

## [8.6.0] - 2025-12-15

### Tornado Diagrams - Sensitivity Visualization

Module for one-at-a-time sensitivity analysis.

### Added

- **Tornado module** (`src/tornado/`)
  - Input range sensitivity calculation
  - Ranked impact visualization
  - JSON/CSV export for charting
  - R sensitivity package validated
- **ADR-021**: Tornado Diagrams architecture decision record

## [8.5.0] - 2025-12-15

### Real Options Analysis - Valuing Managerial Flexibility

Module for options-based project valuation.

### Added

- **Real Options module** (`src/real_options/`)
  - Black-Scholes closed-form pricing
  - Binomial Tree (Cox-Ross-Rubinstein) for American options
  - Greeks: delta, gamma, vega, theta, rho
  - Option types: defer, expand, contract, abandon, switch, compound
  - QuantLib-validated calculations
- **ADR-020**: Real Options architecture decision record

## [8.4.0] - 2025-12-15

### Decision Trees - Sequential Decision Modeling

Module for backward induction analysis.

### Added

- **Decision Trees module** (`src/decision_trees/`)
  - Backward induction solver
  - Node types: Decision, Chance, Terminal
  - Optimal path identification
  - DOT export for Graphviz visualization
  - SciPy/NumPy validated
- **ADR-019**: Decision Trees architecture decision record

## [8.3.0] - 2025-12-15

### Scenario Analysis - Discrete Branching

Module for Base/Bull/Bear analysis with probability weights.

### Added

- **Scenarios module** (`src/scenarios/`)
  - Probability-weighted expected value calculation
  - Discrete mutually exclusive outcomes
  - Per-scenario and aggregate reporting
  - R weighted.mean() validated
- **ADR-018**: Scenario Analysis architecture decision record

### Stats for v8.3.0-v9.0.0

| Module | Tests | Status |
|--------|-------|--------|
| Scenario Analysis | 18 | Pass |
| Decision Trees | 15 | Pass |
| Real Options | 32 | Pass |
| Tornado Diagrams | 19 | Pass |
| Bootstrap | 21 | Pass |
| Bayesian Networks | 23 | Pass |
| **Total New Tests** | **128** | Pass |

---

For versions prior to v8.0.0, see [Changelog Archive](docs/CHANGELOG_ARCHIVE.md)
