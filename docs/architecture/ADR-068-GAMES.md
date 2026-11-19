# ADR-068: Games: a Payoff Table, Its Equilibria, and Tournaments

**Status:** Accepted (the three recommendations). Built so far: the `games:` section (format 5.5.0) and a game played once, with its results as scalars (sections 1 to 3). Next: the `forge game` command and its interfaces (section 4), then tournaments
**Date:** 2026-11-19
**Decided by:** the maintainer, who accepted the three recommendations (2026-11-19); the analysis and the recommendations are Claude's
**For:** roadmap T2-GAME-THEORY. T2-GAME-THEORY-MORE is not part of this.

---

## Context

kinship-protocol (github.com/mollendorff-ai/kinship-protocol) calls Forge its game-theory calculator. Forge works out the payoffs, but it cannot say which strategy is dominated or where the equilibrium is. Its three game models show what is written by hand today:

| Model | What it stores | How it gets there |
| --- | --- | --- |
| `stag_hunt.yaml` | `coop_base` 2, `defect_base` 3, `crossover_c` 1, and `c0_coop_wins` to `c3_coop_wins` (0, 0, 0, 1) | Each strategy's payoff against an opponent who plays each strategy half the time, typed as a formula (`=0.5 * stag_stag + 0.5 * stag_hare`). A cost `c` is taken off each defection. One formula per strategy for each of four values of `c` (`c0_coop`, `c0_defect`, then `c1_`...), and an `IF` the author writes for the flag. |
| `chicken.yaml` | `coop_base` 2, `defect_base` 2.5, `crossover_c` 0.5, flags 0, 0, 0, 1 | The same, for chicken. |
| `axelrod_tournament.yaml` | `grim_base` 2.622, `tft_base` 2.445, `ad_base` 2.212, `ac_base` 2.112, `rand_base` 1.9; defection rates 0.4, 0.3, 1.0, 0, 0.5; `crossover_c` 0.1 | Typed in. They are half the numbers of a tournament Grok ran in December 2025 (`dialogues/grok-thermodynamics.md`): 5 strategies, 200 rounds a match, self-play included. Then `base - defect_rate * c` for four values of `c`. |

None of the three models has an equilibrium in it. Neither does any other model in the repository.

### What the right numbers are, checked before any code

I checked the numbers below by hand and with two outside tools: nashpy 0.0.43 for the equilibria, and the Python `axelrod` library 4.14.0 for the tournament. Each number is asserted in an inline test when this is built, with a comment that names the tool, its version and, for `axelrod`, the runs and the seed.

**Stag hunt** (stag/stag 4, stag/hare 0, hare/stag 3, hare/hare 3). nashpy and the hand calculation agree: two pure equilibria, (stag, stag) and (hare, hare), and one mixed equilibrium in which each hunter plays stag with probability 3/4. With the cost `c` taken off each hare payoff, the mixed equilibrium is stag with (3 − c)/4. It is 1/2 at `c` = 1, the stored `crossover_c`. That is no coincidence. kinship-protocol's flag asks which strategy does better against an opponent who plays each half the time. Here that is the same as asking which side of 1/2 the mixed equilibrium is on.

**Chicken** (swerve/swerve 3, swerve/straight 1, straight/swerve 5, straight/straight 0). Two pure equilibria, (swerve, straight) and (straight, swerve), and a mixed one with swerve at 1/3 (nashpy agrees). With the cost on the straight payoffs, swerve is played with (1 + c)/3. It is 1/2 at `c` = 0.5, the stored `crossover_c`.

**The tournament.** The table Grok printed cannot be right as it stands. GrimTrigger's 5.244 per turn is more than the temptation payoff of 5, the most a player can get in a round. AlwaysDefect's "defection rate" is 2.000. kinship-protocol halved both columns, which gives its stored numbers, but no code for Grok's run was kept. The expected values, worked out by hand and from Forge's planned method (below), are:

| Strategy | Stored (Grok, halved) | Expected value, per round | `axelrod`, 300 seeded runs of each match |
| --- | --- | --- | --- |
| GrimTrigger | 2.622 | 2.596 | 2.599 |
| TitForTat | 2.445 | 2.448 | 2.447 |
| AlwaysDefect | 2.212 | 2.208 | 2.208 |
| AlwaysCooperate | 2.112 | 2.100 | 2.102 |
| Random | 1.900 | 1.909 | 1.909 |

The order is the same, and so are the four `coop_wins` flags (0, 0, 1, 1). The crossover where AlwaysCooperate overtakes AlwaysDefect moves from 0.1 to 0.108: AlwaysDefect defects every time, so each unit of `c` costs it a full unit. Grim's 2.622 is 0.026 above the expected value. That is more than sampling Random would explain, because Grim against Random is close to 3 in every run. The gap cannot be traced without Grok's code.

**The two new games**, also checked with nashpy:

- *Inspection.* An AI complies or cheats, a human audits or trusts. Cheating gains the AI 2 if it is not caught and costs it the penalty (4) if it is. An audit costs the human 1, and undetected cheating costs the human 3. There is no pure equilibrium. In the mixed one the human audits with probability gain/(gain + penalty) = 1/3, and the AI cheats with probability audit cost/harm = 1/3. Double the penalty to 8 and the audit rate falls to 1/5, while the cheating stays at 1/3. This is the known result the roadmap asks the tornado to show.
- *A tournament with noise.* AlwaysCooperate, AlwaysDefect, TitForTat, GrimTrigger, generous TitForTat, win-stay-lose-shift and TitForTwoTats play 200 rounds, and each move is flipped with probability 0.05. In a sample of 60 seeded runs per match with `axelrod`, TitForTat falls from tied first (without noise) to third, and TitForTwoTats is first. The margins in the sample are small (2.271 against 2.245), so the final roster is chosen when this is built, on the exact numbers, and checked against `axelrod` with more runs.

## The decision proposed

### 1. A `games:` section in the model format

A new reserved top-level key, `games:`, holds named games. Each game is either a **table** (a game played once) or a **tournament** (a repeated game). The format version is 5.5.0. A new reserved key changes how a file that has a group named `games` would read (the reasoning is ADR-067's, option 2). So it gets a new schema version, `forge-v5.5.0.schema.json`, as ADR-014 requires. No model that runs today reads differently.

```yaml
_forge_version: "5.5.0"
c: 0            # the cost of a defection, an input like any other

games:
  stag_hunt:
    players: [me, you]
    strategies: [stag, hare]          # one list: every player has these
    symmetric: true                   # a cell gives the first player's payoff
    payoffs:                          # first player's strategy, then the second's
      stag: {stag: 4,       hare: 0}
      hare: {stag: =3 - c,  hare: =3 - c}

  inspection:
    players: [ai, human]
    strategies:
      ai: [comply, cheat]
      human: [audit, trust]
    payoffs:                          # one value per player, in the order of players
      comply: {audit: [0, =-audit_cost],     trust: [0, 0]}
      cheat:  {audit: [=-penalty, =-audit_cost], trust: [=gain, =-harm]}
```

- **A cell is a number or a formula over the model**, as a scalar's formula is. It can read any scalar, table aggregate or included file's value. That is why scenarios, tornado, sensitivity, goal seek and simulate work on a game: they move an input, and the cells move with it.
- **Players: two or more.** With more than two players, `payoffs:` nests one level per player, and the last level holds one value per player.
- **`symmetric: true`** is for two players with the same strategies. A cell then holds one value, the payoff of the player whose strategy is the row. The other player's payoff is read from the mirror cell. kinship-protocol's three games are all symmetric.
- **Errors, each naming what is wrong:** a strategy missing from the table or appearing twice; a cell with the wrong number of values; a strategy name used by two players in a symmetric game that is not the same list; a cell whose formula fails (the cell is named as `stag_hunt.payoffs.hare.stag`).

A **tournament** names a symmetric two-strategy game in the same section as its round, and the strategies that play:

```yaml
games:
  dilemma:
    players: [me, you]
    strategies: [cooperate, defect]   # the first is read as cooperate
    symmetric: true
    payoffs:
      cooperate: {cooperate: =R,      defect: =S}
      defect:    {cooperate: =T - c,  defect: =P - c}

  axelrod:
    tournament:
      round: dilemma
      players:                        # a name, and the built-in strategy it plays
        grim: grim_trigger
        tft: tit_for_tat
        ad: always_defect
        ac: always_cooperate
        rand: {plays: random, cooperate: 0.5}
      rounds: 200                     # or `continuation: 0.99`, not both
      noise: 0                        # the chance that a move is flipped
      self_play: true
```

- **The strategies are built in.** `always_cooperate`, `always_defect`, `tit_for_tat`, `suspicious_tit_for_tat`, `tit_for_two_tats`, `grim_trigger` (defects for good once the other player has defected), `win_stay_lose_shift`, `generous_tit_for_tat` (`forgive:`, the chance of answering a defection with cooperation; 1/3 if not given) and `random` (`cooperate:`, 1/2 if not given). A parameter can be a formula. They behave as the `axelrod` library's strategies of the same name, which makes that library the outside check. A strategy written by the user (a table of moves) is left out of this version.
- **A match ends after `rounds`, or after each round with probability 1 − `continuation`.** With noise, each player's move is flipped with probability `noise`. The strategies see the moves that were played, as in `axelrod`.

### 2. What Forge works out

For a **table**:

1. **Dominated strategies.** A strategy is strictly dominated when another of the player's own strategies pays more against everything the other players can do. Forge removes these one at a time until none is left, and reports the order. Weak dominance is reported in the text but not removed.
2. **Pure-strategy Nash equilibria**, for any number of players: every cell where no player gains by changing strategy alone. This list is always complete.
3. **Mixed equilibria, for two players only:** all of them, by support enumeration (the method nashpy uses). In a degenerate game, where a tie gives a whole line of equilibria, Forge reports the ones it finds. It also says the game is degenerate and names the tie, so an incomplete list is never shown as complete.
4. **Each strategy's payoff against an even mix:** what it pays when every other player plays each of their strategies equally often. This is what kinship-protocol's `coop_base` and `defect_base` compute today.

For a **tournament**:

1. **Each strategy's score:** its expected payoff per round over all its matches (expected total over expected number of rounds), its rank, and its share of moves that were cooperation. kinship-protocol's defection rates are this share.
2. **The match table:** each strategy's payoff per round against each other. This is itself a symmetric game. Forge reports which strategies are **collectively stable** (Axelrod's term). A strategy is collectively stable when no strategy does better against it than it does against itself, so (s, s) is an equilibrium of the match table. A sweep of `continuation` shows grim_trigger becoming stable against always_defect at w = (T − c − R)/(T − P), the threshold kinship-protocol derives by formula (0.5 at c = 0).

### 3. The results are scalars

Every number in section 2 is also a scalar, named under the game, which any formula and any command can read:

| Name | Value |
| --- | --- |
| `<game>.pure_equilibria`, `<game>.equilibria` | How many |
| `<game>.equilibrium_<k>.<player>.<strategy>` | The probability that the player plays the strategy in the k-th equilibrium. Pure ones come first, in table order, then mixed ones by support size. |
| `<game>.equilibrium_<k>.<player>.payoff` | The player's expected payoff there |
| `<game>.dominated.<player>.<strategy>` | 1 if removed as strictly dominated, else 0 |
| `<game>.even_mix.<player>.<strategy>` | The payoff against an even mix |
| `<game>.degenerate` | 1 if the game has a tie that makes the mixed list possibly incomplete |
| `<tournament>.score.<strategy>`, `.rank.<strategy>`, `.cooperation.<strategy>` | As in section 2 |
| `<tournament>.match.<a>.<b>` | a's payoff per round against b |
| `<tournament>.stable.<strategy>` | 1 if collectively stable, else 0 |

So the tornado the roadmap asks for is `forge tornado inspection.yaml inspection.equilibrium_1.human.audit` with `penalty` as an input. kinship-protocol's flag becomes `=IF(stag_hunt.even_mix.me.stag > stag_hunt.even_mix.me.hare, 1, 0)`, and its four copies become four scenarios.

**How they are calculated.** A game is one more cell in the calculator's plan (`Cell::Game`, next to `Cell::Scalar` and `Cell::Column`). It depends on the cells its payoffs read, and its result scalars are its members. A formula that reads a game's result runs after the game. A game whose payoffs read its own results, directly or not, is a cycle, and the existing cycle error names it. A scalar that names a result the game does not have (an `equilibrium_3` when there are two) is an error that names it. If a tornado or a sweep crosses a point where the number of equilibria changes, it stops there with that error. It does not read a different equilibrium under the same name.

`forge calculate` prints the results with the other scalars, but it does not write them into the file. They come from the game, and the file keeps the game.

### 4. The command and its interfaces

- **`forge game model.yaml [--game NAME] [--scenario NAME] [-o FILE]`** prints each game: the dominated strategies, the equilibria, the even-mix payoffs; for a tournament, the ranking, the match table and the stable strategies. `-o` writes the same as YAML or JSON. Its test calls `run` in-process.
- **MCP:** a 21st tool, `forge_game`, with the same arguments. The tool count changes in the MCP test, the server's instructions, `docs/MCP.md` and README.md.
- **forge-wasm:** the same core function, as `game`, so the browser has the same tools as the MCP server. Studio shows nothing new in this change.
- **REST:** nothing. The REST API has only the five core commands (validate, calculate, audit, export, import), and no analysis engine is there.
- **`forge examples`:** one example for each game in the acceptance (`stag-hunt`, `chicken`, `axelrod`, `inspection`, `noisy-tournament`, and the optional `battle-of-the-sexes`), each with numbers worked out outside Forge and asserted in an inline test, as the `bagel_example` tests do.
- **The schema files, `forge schema`, docs/FEATURES.md, docs/cli, the CHANGELOG** and the counts in README.md and VALIDATION.md.

### 5. How it is checked

Every equilibrium and ranking asserted in a test has a comment that says where its number comes from: nashpy (and its version), the `axelrod` library (version, runs, seed), or a hand calculation written out in the comment. For the tournament, Forge's exact value is compared with `axelrod`'s average over many seeded runs, within a stated tolerance, and with a hand calculation where one is short (TitForTat against AlwaysDefect over 200 rounds: 199/200).

## What is left out

- **More than two players in mixed strategies.** With three or more players, Forge reports dominated strategies and pure equilibria, and says it did not look for mixed ones. A five-player public goods game is T2-GAME-THEORY-MORE.
- **Extensive-form games:** moves in turn, what a player knows when, subgame perfection (the ultimatum game, entry deterrence). ADR-019's decision trees cover one decision maker against chance, not two players.
- **Auctions**, and any game with a continuum of strategies (Cournot quantities, Bertrand prices). A price answer from a competitor can be written today as a table over a few named prices, which this does cover.
- **Dominance by a mixed strategy.** A strategy that only a mix of two others beats is not reported as dominated. The report says "dominated by one of the player's own strategies", so it does not claim more.
- **Strategies written by the user** in a tournament, and **evolution:** strategy shares that change from generation to generation (replicator dynamics, T2-GAME-THEORY-MORE).
- **Studio:** no game view in this change.

## The three questions, and what was decided

The maintainer accepted the three recommendations on 2026-11-19: tournaments are exact expected values with no seed; formulas can read a game's results; the names are as written here. The reasoning given for each is kept below.

**1. Exact expected values or a seeded simulation for tournaments? Decided: exact.** The roadmap says "seeded where a strategy is random". The recommendation was the exact expected value instead, with no seed. Every built-in strategy is a small machine: at most three states, and a chance of cooperating in each. Two of them playing, with noise, form a Markov chain of at most nine states. Forge can compute the expected payoff of 200 rounds, or of a match that goes on with probability `continuation`, exactly, by stepping the chain or solving a nine-by-nine system.

| | Exact (recommended) | Seeded simulation |
| --- | --- | --- |
| The same answer every run | Yes, with no seed | Yes, for one seed; a different seed gives a different number |
| Checked against | A hand calculation, and `axelrod`'s average within a tolerance | `axelrod`'s average within a tolerance. Its seeds draw different numbers from Forge's, so no exact match. |
| Tornado and sweeps on `noise` or `continuation` | Smooth | Sampling noise in every bar, unless the runs are many |
| Speed | Microseconds a match | Runs × rounds |
| What it cannot do | Strategies that need a long memory (none of the built-in ones) | Nothing |

With the exact method, `random`, `generous_tit_for_tat` and `noise` are probabilities, not draws. A seeded mode can be added later if a user-written strategy ever needs it.

**2. Can other formulas read a game's results? Decided: yes.** The recommendation was yes (section 3): it is what removes kinship-protocol's hand-written flags. It costs one new cell type in the calculator's plan. The other way is that results can be read only by the commands (`forge tornado` and the like) and never by a formula. That is simpler, but the flags stay in kinship-protocol's files as typed values.

**3. The names. Decided: as written.** `games:`, `players`, `strategies`, `payoffs`, `symmetric`, `tournament`, `round`, `rounds`, `continuation`, `noise`, `self_play`, and the result names in section 3. These are fixed once released (docs/STABILITY.md).

## Consequences

- A model can hold a game and its equilibria next to the numbers its payoffs come from, and every analysis command works on it without change.
- kinship-protocol's three game models get shorter. Their stored results are reproduced (stag hunt and chicken exactly), or the difference is explained (the tournament: up to 0.026, from Grok's run).
- A new format version, 5.5.0, and a 21st MCP tool.
- The calculator gains a cell type. A bug there would affect every model with a game, and none without one. The game is the only new code in the plan.
