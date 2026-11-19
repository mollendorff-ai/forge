# ADR-062: Generated Cases Against Gnumeric and LibreOffice, in CI

**Status:** Accepted (2026-11-14, by merging #206); built (2026-11-14)
**Date:** 2026-11-14
**Decided by:** the maintainer, by merging this ADR (roadmap T1-GENERATED, from the evaluation of rc.23); the design is Claude's, in a session configured for Opus 5.5
**Extends:** ADR-037 (external validation engines), ADR-038 (edge-case discovery) and ADR-057 (the validation suites run on every pull request)

---

## Context

Every case in `validation/gnumeric/tests` is written by hand, so a bug lives where nobody thought to look. `YEARFRAC` from the last day of February to a 31st was wrong until rc.22, and no case had that pair of dates.

`validation/sweep` has run twice by hand, each time over a fixed grid:
- **2026-11-13, date and financial functions:** 9,327 cases, eight wrong answers (fixed in rc.23).
- **2026-11-14, statistical, lookup, math and text functions:** about 5,900 cases, a dozen wrong answers and two crashes (#202).

Each run sends a case through Forge, then through LibreOffice and Gnumeric as an exported workbook, and sorts it by who agrees with whom (`harness.py`). A grid finds what its author thought of putting on it, and it runs only when someone runs it.

**A random draw finds what the grid missed, measured.** `validation/sweep/generate.py` (in this PR, a prototype) draws each function's arguments from a seed, weighted toward the values where spreadsheets go wrong:
- **Dates:** month ends, February 28 and 29, the 31st, 1900, 2100.
- **Numbers:** 0, ±1, ±0.5, halves like 2.5 and 1.005, 1E-12, 1E+12, 1E+15.
- **Integers:** whole numbers in range, but also one past each end, and fractions.
- **Text and data:** multi-byte text, empty text, wildcards; data columns with empty cells.

Three seeds of 40 cases for each of 133 functions ran on 2026-11-14 (15,960 cases, under 30 s a seed with both engines). They found, among others, these candidates the grids had not, each with Forge alone against both engines:

| What | Example | Forge | Both engines |
| --- | --- | --- | --- |
| `CORREL`, `SLOPE`, `INTERCEPT`, `RSQ`, `FORECAST`, `COVARIANCE.P`/`.S` with an empty cell in a column | `CORREL(g.a, g.p)` | an error: the empty cell is dropped from one column only, so the lengths differ | the pair skipped |
| `DATEDIF` with the start after the end | `DATEDIF(DATE(1999,1,31), DATE(1999,1,15), "YM")` | 11 | #NUM! |
| `HOUR`, `MINUTE`, `SECOND` of a negative number | `HOUR(-0.5)` | -12 | 12 (Excel: #NUM!, to check) |
| `INDEX` of an empty cell | `INDEX(h.b, 4)` | an error | 0 |
| `PMT`, `PPMT`, `NPER`, `RATE` at a rate near 0 | `PMT(1e-9, 24, -2.5, -0.227)` | 0.11362885 | 0.11362886 |
| `NORM.DIST` density far in a tail | `NORM.DIST(-179859, -6.66, 1, FALSE)` | an error | 0 |
| `DB` with a life of 0 | `DB(1, 0.093, 0, 8)` | 0 | #NUM! |
| `VDB` with a salvage above the cost | `VDB(1, 3.38, 16, 8.9, 14)` | 0 | #NUM! |
| `MATCH("*", …, 0)` over an empty text | `MATCH("*", g.t, 0)` | 1 | 2 |
| `ACCRINT` with the first interest date before the issue | `ACCRINT(DATE(2027,6,9), DATE(2024,1,16), …)` | 0.9090 | 0.9097 |

The same runs also give two kinds of noise, which must not fail a build:
- **Where the engines disagree with each other** (about 350 a seed), mostly for a reason already known. Gnumeric cannot read `DATEDIF` from a workbook, has no `XMATCH` or `XLOOKUP`, and gives an error for `MAXIFS` and `MINIFS` when nothing matches (LibreOffice and Excel: 0). The 1900 leap year changes dates before March 1900.
- **Where Forge is alone for a reason already recorded:**
  - `DATE` with a year from 0 to 1899 (Microsoft's page, T1-EXCEL-CHECKS).
  - `LARGE` with a fractional k.
  - `VALUE("TRUE")`.
  - `PRICEDISC` on 30/360 month ends (the Excel-checked case in the suite).
  - `TIME` with a fractional minute, `SYD` past the life.

## Decision

1. **The generator lives in `validation/sweep`, beside the grids, and shares their harness.** `generate.py` holds a catalogue: for each function, what each argument is (a number, a positive number, a rate, an integer range, a date, a text, a criterion, a column, a choice), and which may be left out. It uses only Python's standard library, as the rest of `validation/sweep` does. Functions that are not one value from given arguments are not in it:
   - array results (`SORT`, `FILTER`, `UNIQUE`, `SEQUENCE`);
   - random ones (`RAND`, `MC.*`);
   - `LAMBDA`, `LET`, `OFFSET`, `INDIRECT`, `NOW`, `TODAY`.

2. **A seed gives the same cases, every time.**
   - **Per function:** each function's draws come from their own generator, seeded with `"<seed>/<function>"`, so adding a function or a case does not move another function's cases.
   - **Case ids:** a case is `<function>_<seed>_<n>`.
   - **Data columns:** they are drawn from the seed too, and written into the model, so a replay reads the same cells.
   - **Replaying:** `generate.py --seed S --only FN` gives exactly that function's cases. A failure report prints that command and the commit, since a change to the catalogue changes what a seed draws.

3. **Who judges a function.** Each catalogue entry names its judges: both engines by default, or one where the other cannot judge (LibreOffice alone for `DATEDIF`, which Gnumeric cannot read from a workbook, and for `XMATCH` and `XLOOKUP`, which it does not have), with the reason beside it. A narrower gap, such as Gnumeric's error for `MAXIFS` with no match, is a known difference (decision 4). A case is then one of:
   - **agreed:** Forge gives what every judge gives, within 1E-9 relative;
   - **Forge alone:** the judges agree and Forge does not. This is a failure, unless a known difference covers it;
   - **judges split:** Forge agrees with one judge. This is reported, not a failure, as a candidate for T1-EXCEL-CHECKS;
   - **all different:** a failure, unless a known difference covers it.

4. **Known differences are written down, one class of input each, with a reason.** `validation/sweep/known.py` holds them. Each one has:
   - an id;
   - the functions it covers;
   - a test on the case's arguments and values that picks out one class of input (`DATE` with a year from 0 to 1899, `LARGE` with a k that is not whole), never a whole function;
   - the reason;
   - where it is tracked: T1-EXCEL-CHECKS, a roadmap item, or a cited source when the engines are the ones that are wrong.

   The report gives the count each rule absorbed. A rule that absorbs nothing in the weekly run is listed, so a fixed difference does not stay excused.

5. **On every pull request, a fixed seed. Every week, a new one.**
   - **Pull requests:** the `Formulas vs Gnumeric` job in `validation.yml` already has both engines installed. After the suite, it runs `generate.py` with seed 1 and 20 cases a function, about 2,700 cases and about 15 s (3 s as built). The seed is fixed, so a pull request fails only for what it changes, not for an old bug a new draw happens to hit.
   - **Every week:** the Monday run of `validation.yml` draws a new seed, the ISO year and week (202647), with 200 cases a function, about 27,000 cases and 3 minutes (22 s as built). It is there rather than in `weekly.yml`, which installs no engine. A failure fails that run, which is where a new bug is found.
   - **On demand:** `workflow_dispatch` takes a seed and a count, to replay a weekly failure in CI.

6. **What a failure becomes** (T1-TEST-RULE):
   - **A Forge bug:** it is fixed with a handwritten case in `validation/gnumeric/tests` that fails before the fix.
   - **Unsettled until Excel is consulted:** it goes to T1-EXCEL-CHECKS, with a known-difference rule until Excel settles it.
   - **The engines wrong against Excel's documentation:** a known-difference rule that cites it.

   The generator never writes an expected value. Its cases judge only against the engines, and a fact enters the suite by hand.

7. **The candidates in the table above are dealt with before the per-PR step can fail a build.** The build that follows this ADR checks each one by hand. A Forge bug gets a fix and a suite case; anything else gets a known-difference rule. Only then is the step made to fail the job. Until then it reports and passes.

## Built (2026-11-14)

- **`generate.py`** draws the cases, judges them (`JUDGES`: LibreOffice alone for `DATEDIF`, `XMATCH`, `XLOOKUP`), prints the counts, each known difference with how many cases it covered, the ones that covered none, and each failure with the command that replays it. `--fail` makes a failure exit 1.
- **`known.py`** holds 21 rules. Each covers one class of input, gives the reason, and says where it is tracked: mostly T1-EXCEL-CHECKS. Two cite the engines themselves: `NPER` at a rate near 0, where Forge's answer is the exact one (checked with 50-digit decimals), and a difference of large numbers, where double precision leaves no engine as a reference. The `CUMPRINC` rule also notes that Forge's closed form is exact (checked with fractions).
- **One model a batch.** At first `forge calculate` slowed down with the cube of the number of scalars (1,000 took 0.2 s, 4,000 took 8 s, and the 26,600 cases of one model did not finish in 10 minutes), so the cases went in models of 1,000. That was fixed on 2026-11-14 (T1-SCALAR-SCALING): 26,600 scalars calculate in 0.3 s, and a batch is one model again. The weekly batch takes 5 s and the pull-request batch 2 s with both engines (mostly starting them), not the 3 minutes and 15 s measured above.
- **The candidates in the Context table, and those found while building** (seeds 1 to 10 and the week's seed, 202646), are fixed in Forge. Each has a case in `validation/gnumeric/tests/edge/edge_generated.yaml`, or a unit test where the engines cannot check it (the `generated_cases` test groups in `evaluator/`, one in the module of each function). The ones found while building:
  - `NA()` was an empty value, so `IFNA` and `ISNA` missed a lookup that found nothing, and `IFERROR` and `ISERROR` took an empty cell for an error;
  - `COUNTIF` and the other criteria did not compare text with `>` and `<`, and a pattern matched an empty text and a date;
  - `DDB` with a fractional period, or past the salvage;
  - `RANK.EQ` counted 1E-12 as equal to 0;
  - `MOD` gave a remainder larger than the divisor;
  - `RATE` stopped short of the root with a large loan;
  - `CUMPRINC` lost digits at a high rate, and `PMT`, `NPER` and the like lost them at a rate near 0;
  - `RRI` refused a negative ratio over one period;
  - a number as text rounded its binary value, not its shortest decimal form.

## Consequences

- Every pull request is checked on about 2,700 inputs nobody wrote, and the week's run on 27,000 new ones. This is what T1-GENERATED and its acceptance ask for, with the large run in `validation.yml` instead of `weekly.yml`, for the reason above. The roadmap item's acceptance is reworded to match.
- The validation workflow takes a few seconds longer on a pull request.
- A wrong answer the weekly run finds restarts the two weeks before a stable release (docs/STABILITY.md), as any other does.
- `known.py` is a list of excuses, and it can grow. Each rule is one class of input with a reason and a place it is tracked, and the weekly report shows the rules that no longer match. That keeps it honest, not short.
- Agreement with both engines is not correctness: the suite's Excel-checked cases show both engines wrong where Excel and Forge agree. The generator finds disagreements; people settle them.
- Not covered:
  - functions outside the catalogue (decision 1);
  - text formats the one-row-table harness cannot read (a text function over another table's column is measured with `LEN`);
  - the command line and the analysis commands. Property tests (T1-PROPERTIES) are the other half of the plan.
