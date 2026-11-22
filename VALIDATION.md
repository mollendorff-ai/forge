# Validation

How Forge's results are checked, what the checks cover, and how to run each one yourself. Numbers are from 2026-11-12 on Forge main, except those dated 2026-11-14 (rc.23, run by hand).

Forge is a release candidate and an R&D prototype. These checks reduce the chance of a wrong answer; they do not rule it out. Check results that matter independently.

## Summary

| Check | Compares Forge with | Result |
| --- | --- | --- |
| Unit tests (this repo) | Expected values written in the tests | over 3,100 tests and 2 doctests pass |
| [forge-e2e-gnumeric](validation/gnumeric) | Gnumeric, a separate spreadsheet engine, and LibreOffice for 175 cases Gnumeric cannot read | 2,162 formula cases; 2,137 of the 2,138 before ADR-066 pass, 0 fail, 1 skipped, the 168 by LibreOffice included (2026-11-17); the 13 of ADR-066 pass where Gnumeric judges them (2026-11-18), and CI judges the 7 by LibreOffice; the 5 of `edge_fund_cash_flows.yaml` pass in Gnumeric (2026-11-18); the 4 of the ROUND and HOUR fixes pass in Gnumeric (2026-11-18); the 2 of XNPV and XIRR before the first date are checked against Forge only (2026-11-19); CI runs them all on every pull request |
| [forge-e2e-r](validation/r), formulas | R, with Excel rules where R differs | 136 math, date and financial checks pass |
| forge-e2e-r, analytics | R packages `boot`, `stats`, `data.tree`, `derivmkts`, `bnlearn` and base R | 70 of 77 specs pass against R in the Validation workflow; 7 skipped |
| [validation/games](validation/games) | Gambit (pygambit 16.7.0) for equilibria, the axelrod library (4.14.0) for tournaments, R for 2 x 2 closed forms and, from the payoffs alone, the games of many players and their dynamics, egttools (0.2.1) for the dynamics | 25 named games and 8 shipped examples, 300 generated games (2 x 2 to 6 x 6) and 372 tournament values on every pull request; 3,000 games, 1,000 runs a match and 3,000 2 x 2 games on 2026-11-20: 0 disagreements |
| Real Excel (by hand, five rounds) | Microsoft Excel for iPhone, recalculating a workbook written by `forge export` | 18 of 18 financial and date results match (2026-11-06); 35 of 40 edge formulas (2026-11-14), the other five fixed in Forge; all 10 (2026-11-15): 8 calculate alike, and 2 that Excel refuses Forge now refuses too; 13 of 13 comparisons of ADR-065 (2026-11-18), and 69 formulas that measure where Excel sets a sum near zero to 0, which Forge now follows (ADR-066, 2026-11-18) |

The unit tests run on every push. The validation suites, in `validation/` (ADR-057), run in the **Validation** workflow on every pull request and every merge to `main` that changes `src/`, `Cargo.toml` or a suite, and weekly (Actions tab, also runnable on demand) and can be run by hand with the commands below.

## 1. Unit tests

```bash
cargo test --release
```

Includes Black-Scholes and binomial option prices checked against QuantLib reference values, and property tests (`src/lib.rs` (its `properties` tests)): four rules checked on generated models, from a fixed seed on every pull request and from a new seed with 1,000 cases per rule each week. CI runs the tests on stable Rust and strict clippy (`-D warnings`) on every push, and builds on the oldest supported Rust (`rust-version` in Cargo.toml); a weekly job also runs `cargo audit` and the tests on stable and beta Rust.

## 2. Formulas against Gnumeric

Each case is a Forge model with an expected value. A case passes only if both agree with it: `forge calculate`, and Gnumeric's recalculation of the file written by `forge export`. A case whose `libreoffice` key says why Gnumeric cannot read it (Gnumeric 1.12 has no `LET`, `UNIQUE` or `XLOOKUP`, gives #VALUE! for `DATEDIF` read from an xlsx file and for `ISFORMULA` of a literal, and differs from Excel on a comparison at 15 digits and on two financial results) is recalculated by LibreOffice instead, run with a profile that recalculates every formula on load: the workbook caches Forge's results, which LibreOffice would otherwise show.

```bash
cargo build --release
cd validation/gnumeric
cargo run --release      # runs ../../target/release/forge; FORGE_BIN=/path/to/forge for another
```

Needs `ssconvert` (package `gnumeric`) and LibreOffice 24.8 or later (`soffice`; on Ubuntu 24.04 from noble-backports). Result on 2026-11-17 (after rc.27, with VLOOKUP and HLOOKUP written as formulas and two numbers compared to 15 digits, ADR-065): 2,138 cases, of which 168 are recalculated by LibreOffice (26.2); 2,137 passed, 0 failed, 1 skipped. On 2026-11-18 the 13 cases of ADR-066 (`edge_subtraction.yaml`) were added, 2,151 in all, of which 175 are recalculated by LibreOffice: with LibreOffice 24.2, which the suite refuses, every case Gnumeric judges passed (1,975), the 6 new among them. The same day the 5 cases of `edge_fund_cash_flows.yaml` were added (2,156 in all): `XIRR` and `XNPV` on the 26 dated cash flows of `examples/fund`, one set out of date order, with expected values from Python; Gnumeric 1.12 and Forge passed all 5. Every case is also checked after `forge export` and `forge import`, so a formula that does not survive the round trip fails. 40 cases are checked against Forge only, each with a written reason in its spec (`gnumeric_skip`), among them `PRICEDISC` and `ACCRINT` results where Gnumeric 1.12 and LibreOffice 26.2 both differ from Excel, which was checked by hand (section 5). With every cached scalar result written 1,000 too high, the 144 cases of two spec files (27 of them in LibreOffice) still all passed, so both engines recalculate rather than read the cache.

## 3. Math, date and financial functions against R

The R validators send each formula through `forge calculate` and compare the answer with R. Where R and Excel differ, Excel is the reference (`ROUND` rounds half away from zero; `NETWORKDAYS` counts both end dates).

```bash
cargo build --release
cd validation/r
FORGE_BIN=../../target/release/forge bash validators/r/run_all.sh
```

Needs R with `jsonlite`. Result: 136 checks pass (rc.23, 2026-11-14; the script now fails if any validator fails).

## 4. Analytics against R

Monte Carlo, bootstrap, tornado, sensitivity, decision trees, real options and Bayesian networks: the runner builds a Forge model from each spec, runs the Forge command and compares the output with the matching R validator.

```bash
cd validation/r
cargo run --release -- --all      # or --binary /path/to/forge
```

| Category | Specs | Pass | Not compared, and why |
| --- | --- | --- | --- |
| Monte Carlo | 12 | 10 | 2 use the exponential distribution, which Forge does not have |
| Bootstrap | 12 | 12 | Percentile, basic and BCa intervals |
| Tornado | 8 | 7 | 1 has an input whose low equals its high, which Forge rejects |
| Sensitivity | 10 | 6 | 3 are elasticity (no Forge command); 1 is a single point |
| Decision trees | 10 | 10 | |
| Real options | 15 | 15 | Price only; Forge's command does not print Greeks |
| Bayesian networks | 10 | 10 | |

Running without `--expected-only` on a machine without `data.tree`, `derivmkts` or `bnlearn` reports those specs as errors, not passes (`derivmkts` is optional). `--expected-only` compares decision trees, real options and Bayesian networks with the expected values in the spec and does not run R. Bootstrap, tornado and sensitivity need a Forge build with machine-readable `--output` (included from PR #27).

## 6. Games against Gambit, axelrod and R

`validation/games` (ADR-068, roadmap T2-GAME-THEORY-E2E) runs `forge game -o` and compares it with engines Forge shares no code with. Equilibria: Gambit (pygambit 16.7.0, its pure and mixed solvers) on 25 named games (kinship-protocol's stag hunt, chicken and dilemma at four costs each, the inspection game, battle of the sexes, the acquisition's sealed bids in two scenarios, matching pennies, rock-paper-scissors, three steps of dominance, degenerate games), on the examples that ship, and on seeded random games from 2 x 2 to 6 x 6, with ties on purpose. Tournaments: the axelrod library (4.14.0), every pair of the nine strategies in three settings, as averages of seeded matches within 4.5 standard errors (the delta method's, for the expected total over the expected length). R: the closed forms of 2 x 2 games, since R has no package that finds equilibria.

```bash
cargo build --release
pip install -r validation/games/requirements.txt
cd validation/games
FORGE=../../target/release/forge python3 -I run.py --seed 1 --games 300 --runs 200 --fail
```

A disagreement fails the run unless Forge reports the game as degenerate, and a degenerate report has to be proved by the suite (a pure strategy with two best replies, or an equilibrium of Gambit's with a spare best reply), so it cannot excuse one it does not describe. Result on 2026-11-20: a pull request's run (seed 1, 300 games, 200 runs a match) takes 30 s and has 0 failures; the weekly run's size (seed 202647, 3,000 games with 8,759 equilibria, 1,000 runs a match, 3,000 2 x 2 games of which 702 have a mixed equilibrium) has 0 failures. In the generated games Gambit finds equilibria Forge does not list only in games Forge reports as degenerate (a line of equilibria, of which Gambit lists the corners). Each check was also tried on a deliberately wrong report and failed it.

Symmetric games of three to five players in the count form (ADR-073): the public goods game of five players below, at and above r = n, and seeded games of two or three strategies, each table expanded again by the suite and compared with Forge's cell by cell, and its pure equilibria with Gambit's `enumpure_solve` (`counted.py`; 303 games and 3,376 pure equilibria, 0 failed, seed 202611, on 2026-11-21).

The mixed equilibria in which every player plays alike, of symmetric games of two strategies (ADR-073, question 4; `symmetric.py`): the volunteer's dilemma of 2 to 8 players against its closed form 1 − (c / b)^(1 / (n − 1)), the public goods game at r = 5 (degenerate: no mix listed, the tie named), and seeded games of three to five players, each mix compared with the exact roots of the population function (sympy's real-root isolation over the game's fractions) and with Gambit's `enumpoly_solve`, and each payoff and `stable` worked out with fractions. On 2026-11-21, seed 31: 158 games, 150 mixes, each an exact root, 0 failed; Gambit found 145 of them and none that Forge lacks (its method is numerical; two it missed in another run, at 0.9475 and 0.9145, are exact roots, g = −1.3e-15 and 1.3e-14 there). Each check was tried on a deliberately wrong Forge: with the stability turned around, a run of 18 games had 14 failures; with the last root dropped wherever there are two, a run of 38 games (seed 202611) had 12 failures, the exact roots and Gambit each naming the missing one.

The dynamics of a symmetric game (ADR-073, question 3; `dynamics.py`): every generation of Forge's path against egttools 0.2.1, a library for evolutionary game theory with a C++ core. egttools gives the replicator gradient x (f − f̄) of a game of n players from its own payoff table; the next shares must be Forge's shares plus that gradient over f̄ + b, within 1e-9, and Forge's average payoff in each generation must be the one worked out here exactly, with fractions. The cases: the bagel's price war across the city, in the plan and with loyal regulars; hawk-dove with V = 4 and C = 6 from either side, which must end at V / C = 2/3; the stag hunt from 0.76 and 0.74, which must end at all stag and all hare; the volunteer's dilemma of five players, at its closed form; rock, paper, scissors of three players, which stays at the even mix; the two examples with their scenarios; seeded games of two to five players and two or three strategies in the count form; and a baseline too low, which must be refused with the strategy named. On 2026-11-21, seeds 2, 31 and 202647 with 300 drawn games each: 313 games and 14,420 generations each, 0 failed. Before release the suite found a fault: the shares were divided by f̄ + b, which is their sum only while they add up to exactly 1, so where f̄ < 0 a rounding grew each generation and one share passed 1 (the test `the_shares_add_up_to_1_in_every_generation`). R works the same cases out again on its own (`many_players.R`, ADR-073): from each of Forge's generations its own next shares and average payoff, within 1e-9, and its own path from the start where the end is known by hand; the symmetric mixed equilibria of games of two strategies as the roots of the population function by `polyroot`, each with its stability and payoff; the volunteer's dilemma's closed form for 2 to 8 players; the public goods game's 1, 32 and 1 pure equilibria at r = 1.6, 5 and 6. On 2026-11-22, seed 1 with 300 games: 86 games (the bagel's price war among them), 3,713 generations, 70 mixes, 0 failed; seeds 2, 31 and 202647 with 3,000: 624 games and 14,733 generations each, 408 to 442 mixes, 0 failed. Each of its checks was tried wrong on purpose: a step with 0.001 added to the baseline (126 failures), the stability the other way round (70), the closed form with c 0.01% higher (7), 31 pure equilibria expected at r = 5 (1), and g shifted by 0.01 (54). Each check of `dynamics.py` was tried on a deliberately wrong Forge: with every split of the others counted once, not by its number of orders, a run of 43 games had 135 failures; with half the baseline in the step, a share went below 0 and Forge refused a seeded game, which stops the run.
