# Games against Gambit, axelrod and R

Forge's games (ADR-068) are checked here against engines Forge shares no code with, as `validation/gnumeric` and `validation/r` check the formulas and analytics (ADR-057). Roadmap item T2-GAME-THEORY-E2E.

| Part | Forge | Compared with |
| --- | --- | --- |
| Named games (`specs.py`) | `forge game -o`, on a model the suite writes and on the shipped examples | [Gambit](https://gambitproject.readthedocs.io) (pygambit 16.7.0): `enumpure_solve` and `enummixed_solve` |
| Tournaments (`tournaments.py`) | the exact match values of the nine strategies, in three settings, and the examples' tournaments | the [axelrod](https://axelrod.readthedocs.io) library (4.14.0), averages of seeded matches |
| Generated games (`generated.py`) | seeded random games, 2 x 2 to 6 x 6, ties on purpose | Gambit, as the named games |
| Games of many players (`counted.py`) | symmetric games of three to five players in the count form (ADR-073): the public goods game and seeded games | the table expanded again here, cell by cell, and Gambit's `enumpure_solve` |
| Symmetric mixes (`symmetric.py`) | the mixed equilibria in which every player plays alike, of symmetric games of two strategies (ADR-073): the volunteer's dilemma of 2 to 8 players, the public goods game at r = 5, and seeded games of three to five players | the closed form 1 − (c / b)^(1 / (n − 1)); the exact roots of the population function (sympy); Gambit's `enumpoly_solve`, every one it finds being Forge's; each payoff, and `stable` from the sign of g′, worked out here with fractions |
| 2 x 2 closed forms (`closed_forms.R`) | seeded random 2 x 2 games | R: the formulas for the mixed equilibrium and the cell check for the pure ones |

R has no package that finds the equilibria of a game, so it checks only the case with a formula. Gambit is a separate implementation with its own pure, mixed and LCP solvers. nashpy, which the unit tests were checked with, runs the same support enumeration as Forge, so it is not independent of the method and is not used here.

## Running it

```bash
cargo build --release
python3 -m venv /tmp/games && /tmp/games/bin/pip install -r validation/games/requirements.txt
cd validation/games
FORGE=../../target/release/forge /tmp/games/bin/python -I run.py --seed 1 --games 300 --runs 200 --fail   # as a pull request runs it (30 s)
FORGE=../../target/release/forge /tmp/games/bin/python -I run.py --only generated --seed 202647 --games 3000   # replay one part
```

`--only` takes `specs`, `tournaments`, `generated`, `r`, `counted` and `symmetric` (at most 100 drawn games: `enumpoly_solve` takes seconds a game of five players). The R part needs `Rscript` with `jsonlite`; pygambit builds from source where there is no wheel for the Python version (a few minutes). The `Games vs Gambit, axelrod and R` job in `.github/workflows/validation.yml` runs seed 1 on every pull request and push that changes `src/` or `validation/`, and on Mondays, and on demand, the week's seed (ISO year and week) with 3,000 games and 1,000 runs of each match. The same seed, counts and library versions give the same games and the same numbers.

## What fails the run

For each table, in `tables.py`:

1. Every equilibrium Forge lists is one: no player gains by a change, and the payoffs it reports are the payoffs of the profile.
2. The pure equilibria are Gambit's, whatever the ties. Forge's list of them is complete.
3. Every mixed equilibrium Gambit finds, Forge finds, **unless Forge reports the game as degenerate** (its `ties` is not empty): a tie can give a line of equilibria, and the report says its list may be incomplete. Gambit lists the corners of a line.
4. Every equilibrium Forge lists, Gambit lists, with the same exception.
5. A game not reported as degenerate has an odd number of equilibria.
6. The degenerate report is not free: a game Forge reports as degenerate must be shown to be, by a pure strategy with two best replies or by an equilibrium of Gambit's with a spare best reply (exact, in fractions), and a game with that proof must be reported. So "degenerate" cannot excuse a disagreement it does not describe.
7. The strategies removed as strictly dominated (in order, each valid when removed, none left) and those reported as weakly dominated are the ones an independent check finds; the payoff against an even mix is the mean over the opponent's strategies.

A disagreement that is not a reported degenerate game fails the run and prints the game's payoff table, with the command that replays it.

For each tournament, in `tournaments.py`: Forge's match value for every ordered pair of strategies, and each strategy's share of cooperation, is within 4.5 standard errors of axelrod's average. A match is played `runs` times with `axl.Match` (one at a time: its `Tournament` spawns processes), seeds 0 to `runs - 1`. The average is the expected total over the expected length, as in ADR-068, and the standard error is the delta method's. Forge's score, rank and stable list are worked out again from its own match table. The settings: 200 rounds with noise 0.05; a continuation of 0.9 with noise 0.02; 200 rounds with no noise. The strategies are `always_cooperate`, `always_defect`, `tit_for_tat`, `suspicious_tit_for_tat`, `tit_for_two_tats`, `grim_trigger` (axelrod's `Grudger`), `win_stay_lose_shift`, `generous_tit_for_tat` (`GTFT(p=1/3)`) and `random` (`Random(0.5)`).

## What a failure becomes

- **A Forge bug:** it is fixed with a unit test in the file that has the bug, which fails before the fix, and the case goes into `specs.py` if it is a named game.
- **A tolerance exceeded by chance in a tournament:** with the same seeds a run does not change, so this is a failure of the weekly run's new seed only. Replay it with more runs (`--runs 4000`) before calling it Forge's.
- **Gambit wrong:** report it to Gambit, and keep the case here as a known difference with the issue named. The suite has none today.
