# ADR-069: One Model Validator, Without a JSON Schema Engine

**Status:** Superseded by ADR-072 (2026-11-21): `jsonschema` checks every model again
**Date:** 2026-11-19, built 2026-11-20
**Decided by:** the maintainer (one hand-written validator on every platform, `jsonschema` kept only to check it, the schema files still published), who chose option 1 on 2026-11-20; the analysis and the recommendation are Claude's
**Closes:** roadmap T1-ONE-VALIDATOR

> **Note (2026-11-21):** ADR-070 replaced the 2,000,000-byte budget this ADR weighs `jsonschema` against with a 3,000,000-byte ceiling from a load-time target, and says size alone is no reason to replace a library with code of our own. The size argument below no longer binds; the others (one validator, the same messages on every surface, the crates `jsonschema` brought) still stand. Whether to keep the reader is roadmap T1-REVISIT-VALIDATOR.

---

## Context

Every model Forge reads goes through one function, `parser::schema::validate_against_schema`, called from `parser::model` (`src/parser/model.rs:20`). The command line, the MCP server, the REST API and the browser (`forge-wasm`) all read a model that way. The function runs Forge's own checks first (`check_short_form`, `check_period_columns`, `check_empty_cells`, `check_date_scalars`, `check_formats`, `validate_v1_0_0_no_tables`), then the `jsonschema` crate against the schema file of the model's `_forge_version`, one of six built in with `include_str!`. `schema_errors.rs` turns what `jsonschema` reports into Forge's words.

In the browser that check costs too much. Measured on main (538c8f5), 2026-11-19: it is 776 KB of the 1,973,854 bytes of the gzipped module, 39%. PR #296 made it 87 KB smaller by building its crates for size, and the module is now 1,887,038 bytes against a budget of 2,000,000 in `ci.yml`. With the check off on the same commit it is 1,202,754 (2026-11-20), so the check now costs 684 KB. In the browser build, 53 of 236 crates are reachable only through `jsonschema`: `jsonschema`, `jsonschema-regex`, `jsonschema-value`, `referencing`, `fancy-regex`, `fluent-uri`, `fraction`, `email_address`, `ahash` (and with it `getrandom` 0.3, which `Cargo.toml` adds for the browser only for `ahash`), and the whole `idna` and ICU stack that the `idna` feature brings in. The `regex` crate stays: the Excel formula translation uses it.

The check cannot be dropped in the browser. With it off, 21 of 2,334 library tests fail. Nine are the schema module's own. The other twelve catch mistakes nothing else catches: a misspelt `formula:` key (the scalar is never calculated), a table column that is not a list, a null in a numeric column, a format Forge does not read, a date where it cannot be, a misspelt scenario probability, unknown keys in general, and refusals in `forge upgrade`. A validator in the browser only would have to agree with `jsonschema` for ever, in a project whose promise is the same answer everywhere.

### What the schema files use

Read from the files on 2026-11-19 (`schema/forge-v1.0.0.schema.json` and `forge-v5.0.0` to `forge-v5.4.0`; `forge-v5.5.0`, added by ADR-068 the next day, has the same keywords and one more, `minProperties`), with the keywords that only describe (`title`, `description`, `examples`, `default`, `$schema`, `$id`) left out. All six are JSON Schema draft 07.

| Keyword | Where | How |
| --- | --- | --- |
| `type` | all | One type, or a list (`["number", "object"]`, `["array", "string", "object"]`) |
| `properties`, `required` | all | |
| `additionalProperties` | all | `false` (19 places in 5.4.0) or a schema (14 places) |
| `$ref` | all | Always `#/definitions/<name>`, never another file, never with other keywords beside it |
| `enum` | all | Strings only |
| `anyOf`, `oneOf`, `not` | all | |
| `allOf`, `if`, `then`, `else` | 5.1.0 on | The short form (ADR-051) and the period columns (ADR-053) |
| `items` | 5.0.0 on | One schema, never a list of them |
| `minItems`, `maxItems` | 5.0.0 on | |
| `minimum`, `maximum` | 5.0.0 on | Inclusive (no `exclusiveMinimum`) |
| `pattern` | all | Six patterns: `^=.+`, `^=(SUM\|AVERAGE\|MAX\|MIN\|COUNT\|PRODUCT\|SUMIF\|AVERAGEIF\|COUNTIF)\(.+\)`, `^\d{4}-\d{2}(-\d{2})?$`, `^[0-9]{4}-(0[1-9]\|1[0-2])(-(0[1-9]\|[12][0-9]\|3[01]))?$`, `^[0-9]{4}-[0-9]{2}-[0-9]{2}$`, `^[0-9]{4}-[0-9]{1,2}(-[0-9]{1,2})?$` |
| `format` | 5.0.0 on | `date` only, on the items of `DateArray` |

Seventeen keywords. Nothing else: no `const` (only `forge.schema.json` has it, which Forge does not read), no `patternProperties`, `propertyNames`, `uniqueItems`, `contains`, `dependencies`, `multipleOf` or string lengths.

### What `jsonschema` 0.60 does that a replacement must do too

Read from its source (0.60.2, the version in Cargo.lock):

- **`format: date` is checked.** For drafts 4, 6 and 7, `jsonschema` treats `format` as an assertion unless told otherwise, and Forge does not tell it otherwise. A date is `YYYY-MM-DD`, ten characters, with a real month and day, leap years counted. So `DateArray` refuses `"2025-01"`, although the schema's own example lists it: a column of months fits as text instead. An editor whose JSON Schema engine treats `format` as a note can disagree with Forge here.
- **An integer is a number with no fraction**, so `1.0` is an integer (draft 6 on).
- **A pattern is searched for anywhere in the text**, and `\d` is `[0-9]`, as in ECMA-262: `jsonschema-regex` translates it before compiling with Rust's regex, whose `\d` would also take other scripts' digits. `.` is Rust's: any character but `\n`.
- **The error report** that `schema_errors.rs` reads: each error's place in the model (`/loan/payment`), its place in the schema with `$ref` followed (`/definitions/TableColumn/anyOf`), its kind (an unknown key with the keys, a type with the types expected, a pattern, an enum with its options, a minimum, a maximum, `minItems`, a required key), and for `anyOf` and `oneOf` the errors of each alternative. A kind `schema_errors.rs` does not word itself (`maxItems`, `format`, a pattern that is not a formula's, `oneOf` with more than one match) is printed in `jsonschema`'s own words, with the value masked.

## The two ways to write it

Either way it is one validator, in the main crate, used by everything that reads a model, and `jsonschema` checks it in tests.

### Option 1: a reader of the schema files

A small evaluator for the seventeen keywords above, with the semantics of the previous section, that reads the same six schema files. It returns errors with the facts `schema_errors.rs` reads today (the place in the model, the place in the schema, the kind, the alternatives' errors), and `schema_errors.rs` moves onto them with its words unchanged.

- **The files stay the one statement of the format.** A change to a schema file is enforced by Forge the moment it is made; there is nothing to keep in step.
- **A keyword it does not read is an error, not a pass.** If a schema file gains a keyword outside the seventeen, every model of that format is refused with a message naming it, and a test that reads all six files fails first.
- **What `jsonschema` checks in tests is seventeen keywords**, each with a definition in the draft, not six grammars written out twice.
- **About 400 lines and their tests**, from a first sketch. The schema text is already in the browser module (`forge-wasm` serves it as `schema`), so it adds no data there. Each schema is parsed once per process, not once per model as today.

### Option 2: the format written again in Rust

A function per definition per format version (`ScalarMapping`, `TableColumn`, `MonteCarloConfig` and the rest, six times with the differences between versions), each checking its keys and shapes directly and writing its own messages. The schema files stay, for `forge schema` and the editors.

- Messages can be written beside each check, and the compiler checks the code's shape. Nothing is parsed at run time.
- **There are two statements of the format**, the Rust and the JSON, and only the test against `jsonschema` keeps them together. Every change to the format is made twice. 5.4.0's schema is 31 KB; the six are 139 KB. A rule written down in one place and not the other is the drift the roadmap item names, moved into this repository.
- Several thousand lines, by the size of the definitions. Most of it is the same rules again for each version.

### Not options

- **Keep `jsonschema` natively and use another validator in the browser:** two validators, which is what the roadmap item rules out.
- **Make `jsonschema` smaller:** #296 built its crates for size (87 KB). Turning off its `idna` feature, which Forge enables and its schemas never need, saves 55 KB more (1,831,664 bytes, measured 2026-11-20). Both together are about 140 KB of the 684 KB the check costs; the rest is the crate, which has no feature that leaves out its other drafts and formats.

## Decision

**Option 1, a reader of the schema files.**

1. **Where.** A module beside `schema.rs` (`src/parser/schema/validator.rs` or similar, split by what it does if it nears 1,500 lines). `validate_against_schema` keeps its signature and its own checks, and calls the reader instead of `jsonschema`. Nothing that calls it changes.
2. **What it reads.** The seventeen keywords of the table, with draft 07's meaning as `jsonschema` 0.60 applies it (the list in Context): `format: date` asserted, `1.0` an integer, `\d` as `[0-9]`, `$ref` only to `#/definitions/`. An unknown keyword, a `$ref` elsewhere or a pattern it cannot read makes every model of that format refused, with a message that names it, and a test that reads the six files fails first.
   *As built:* the patterns are not compiled with the `regex` crate, which was to stay in the browser for the Excel translation. Measured on 2026-11-20, nothing else in the browser calls `regex`: it reached the module only through `jsonschema`, and alone it was 277 KB of it (1,488,393 bytes gzipped with it, 1,211,577 without). The patterns are matched by a matcher of the reader's own (`src/parser/schema_pattern.rs`): literal characters, `.`, `^`, `$`, classes, groups with alternatives, `?`, `*`, `+` and `{n,m}`, checked against the `regex` crate in its tests. It loops over a repeated character and allows a group only a bounded number of repetitions, so a long text in a model never makes it go deeper.
3. **The messages.** Every line `schema_errors.rs` writes today is written word for word, in the same order. The lines that today are `jsonschema`'s own words get Forge's: `'t.x' has more than 2 entries`, `'t.month[0]' must be a date (YYYY-MM-DD), not the text "2025-13-01"`, a non-formula pattern as what the text must look like, and `'x' fits more than one of the forms allowed here`. The tests in `schema_errors.rs` and the 21 tests pass unchanged.
4. **The proof, with `jsonschema` as a dev-dependency.** A test runs both validators on (a) every model under `examples/` and every model written in a test that reaches `validate_against_schema`, and (b) bad models generated from each example by fixed mutations at every key path: a key misspelt, a key added, a required key dropped, a value replaced by null, true, a number, text, a list, a mapping, a formula without its `=`, a month, an impossible date, and a number just past each `minimum` and `maximum`. They must accept and refuse the same models. Generation is deterministic, so a failure names the model and the mutation.
   *As built:* both must also give the same lines (sorted, since the order is `jsonschema`'s to change): `jsonschema`'s errors are turned into the reader's and worded by the same code. In a test build `validate_against_schema` runs this check on every model it is given, so each of the library tests' models is checked too. The generated corpus is 78,838 models (the examples as each of the seven formats, and changed at every place) and gives every kind of error the schemas can (`the_reader_and_jsonschema_refuse_the_same_generated_models`); small schemas check each keyword and each order of errors against `jsonschema` on their own. One form the six files never use, `additionalProperties: false` without `properties`, `jsonschema` reports apart (as a false schema): the reader does not read it, rather than word it differently.
5. **The browser.** The native and WebAssembly comparison (`forge-wasm/expected.json`, `forge-wasm/tests/node/compare.mjs`) gains a set of refused models with their messages, so CI checks that the browser refuses each one with the same words as the command line.
6. **One time, in the pull request:** the messages of the old path (`jsonschema` plus today's `schema_errors.rs`) and the new one are compared on every refused model of point 4, and the differences listed. Only the lines of point 3 may differ.
   *As built:* on the 78,838 models of point 4 (44,473 refused by main's `jsonschema` path), 2,263 messages differ, each only in the lines of point 3: 3,241 lines for a date scalar, 54 for a period's start, 8 for too many correlation variables and 4 for a tournament with fewer than two players (`value has less than 2 properties`, now `needs at least 2 entries`). Every other line is the same, in the same place.
7. **Dependencies.** `jsonschema` moves to `[dev-dependencies]`; its two target entries in `[dependencies]` go, and so does `getrandom_03`. The `[profile.wasm.package.*]` entries of #296 for crates no longer in the browser build go. The size budget in `ci.yml` is lowered to the measured size plus 100 KB.

## Consequences

- The browser module loses the 53 crates. The check off gives 1,202,754 bytes; the acceptance (at least 500 KB smaller, at most 1,387,038) leaves 184 KB for the reader and the messages, which by their size in lines should take a small part of it. Measured as in #296. *As built:* 1,261,203 bytes gzipped (2026-11-20), 678,737 less than main's 1,939,940 (which already had the games of ADR-068); before the games were merged the check cost 26 KB over the same build with it off. The budget in `ci.yml` is 1,362,000.
- The command line, the MCP server, the REST API and the browser refuse the same models with the same words, because they run the same code. A model's acceptance does not change on any of them; point 4 is the test of that.
- The schema files stay where they are and are still published (`forge schema`, `forge_schema`, `editors/vscode/schemas`). Changing one now changes what Forge enforces directly, through the reader. A change that uses a new keyword needs the reader extended first, and the test of point 2 says so.
- The native binary loses `jsonschema` too, and with it about 50 crates from a release build.
- Forge now owns its reading of draft 07 for seventeen keywords. If a later `jsonschema` reads one of them differently, the dev-dependency test fails, and the difference is decided rather than taken.

## The 21 tests

Reproduced on main (6a16aeb) with `if std::env::var_os("NO_SCHEMA").is_some() { return Ok(()); }` at the top of `validate_against_schema` and `NO_SCHEMA=1 cargo test --lib`: 2,313 passed, 21 failed. The early return also skips Forge's own checks before `jsonschema`, so some of these test those checks (the format version, the empty cells, the short form). They are the minimum specification either way.

- `cli::commands::upgrade::tests::`: `a_1_0_file_with_scalars_is_restructured_and_the_loss_of_comments_is_said`, `a_file_that_does_not_convert_is_left_unchanged`, `a_file_that_is_not_a_mapping_is_refused`
- `parser::arrays::tests::empty_cells::`: `an_earlier_format_says_which_one_has_the_empty_cell`, `only_a_column_of_numbers_has_empty_cells`
- `parser::model::tests::test_null_in_numeric_array_error`
- `parser::schema::tests::`: `test_missing_forge_version`, `test_schema_still_rejects_invalid_column`, `test_unsupported_version`
- `parser::schema_errors::tests::`: `a_bare_formula_where_a_scalar_belongs_says_how_to_write_it`, `a_misspelt_key_is_named_with_the_key_it_is_close_to`, `a_misspelt_section_or_scenario_key_is_named`, `each_usual_mistake_says_what_is_expected`, `many_problems_are_counted_after_ten`, `what_the_parser_would_drop_is_an_error`
- `parser::tests::short_form::short_form_in_a_5_0_0_file_says_which_format_to_use`
- `parser::tests::test_parse_table_column_not_array`
- `parser::variables::tests::number_formats::a_format_forge_does_not_read_or_on_the_wrong_value_is_refused`
- `parser::variables::tests::scalar_dates::a_date_is_refused_where_it_cannot_be`
- `scenarios::config::tests::scenario_forms::a_misspelt_probability_is_still_caught`
- `tests::config_strictness::test_schema_rejects_unknown_scalar_keys`

## Update, 2026-11-20: model format 5.5.0

ADR-068 added `schema/forge-v5.5.0.schema.json` while this was built. It uses one keyword beyond the seventeen, `minProperties` (a tournament's `players` needs two entries), and the reader refused every 5.5.0 model until it read it, as designed: a keyword it does not read makes every model of that format refused. It is now the eighteenth. `jsonschema` runs it after `maxItems` and before `required`, on mappings only, and the reader does the same. The words are `'games.axelrod.tournament.players' needs at least 2 entries`, as for a list. The reader is in two files, `schema_reader.rs` (reading a schema) and `schema_check.rs` (checking a model against it), since one would pass the limit of 1,500 lines.
