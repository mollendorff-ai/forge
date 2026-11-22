# ADR-073: Games of Many Players, and Strategies That Spread

**Status:** Accepted (2026-11-21). Questions 1 and 2 as recommended; questions 3 and 4 decided after a second look, as written. Built: steps 1 to 5 (step 4 with `polycool` in place of `roots`: see the note under question 4; step 5 with the two changes in the note under question 3). Not built: Studio's view of the path and of a game of many players
**Date:** 2026-11-21
**Decided by:** the maintainer (2026-11-21), who asked of questions 3 and 4 whether each is good for people who use Forge for game theory or is scope creep, and whether a library does it; the analysis is Claude's
**For:** roadmap T2-GAME-THEORY-MORE. It extends ADR-068 and changes none of its decisions

---

## Context

ADR-068 gave Forge games played once and tournaments. Two of its limits were left to T2-GAME-THEORY-MORE on purpose: mixed equilibria beyond two players, and any dynamics.

kinship-protocol v0.6.0 used rc.31 for its five-player public goods game (`models/public_goods.yaml`, five AIs who each contribute or keep). The roadmap item records what Forge already does and what it lacked.

**Already right.** Forge's pure equilibria for five players agree with Gambit (pygambit 16.7, `enumpure_solve`) and a hand enumeration in all five of its scenarios: 1, 1, 32, 1 and 1 equilibria. The 32 is the degenerate case where every profile is one. Nothing in the solver is limited to two players except the mixed equilibria. `src/games/table.rs` holds a game as one list of profiles, and dominance, pure equilibria and the even mix read it for any number of players.

**What it lacked:**

1. **`symmetric: true` takes exactly two players.** `games.pg is symmetric, which needs two players with the same strategies`. So a game whose five players have the same payoffs is written as 32 cells of five payoffs each, 160 values, produced by a script. What the author knows is shorter: a player's payoff depends on its own strategy and on how many of the others play each strategy.
2. **A tournament's round must be a symmetric two-player game.** So the repeated public goods game cannot be written.
3. **Replicator dynamics**, strategy shares that change from one generation to the next, have no place to go. They would sit beside kinship-protocol's `predator_prey.yaml`.

### The numbers, before any code

The public goods game in its usual form has *n* players. Each has an endowment *e* and either contributes it or keeps it. The pot is multiplied by *r* and shared equally:

- contribute: *r e* (*k* + 1) / *n*, where *k* is how many of the other players contribute
- keep: *e* + *r e k* / *n*

Keeping minus contributing is *e* (1 − *r*/*n*) whatever the others do. So:

- with *r* < *n*, keep strictly dominates contribute and the one equilibrium is everyone keeping;
- with *r* > *n*, contribute dominates and the one equilibrium is everyone contributing;
- with *r* = *n*, every player is indifferent in every profile, so all 2⁵ = 32 profiles are equilibria.

These are the 1, 1, 32, 1, 1 the roadmap records. This hand calculation is the check for the example below, next to Gambit.

## The four questions, and what was decided

### 1. How a symmetric game of more than two players is written

**Decided: a payoff for each of a player's own strategies, written as a formula over how many of the other players play each strategy.**

```yaml
_forge_version: "5.5.0"
endowment: 10
r: 1.6

games:
  pg:
    players: [a, b, c, d, e]
    strategies: [contribute, keep]
    symmetric: true
    payoffs:                           # one player's payoff, for its own strategy
      contribute: =r * endowment * (pg.others.contribute + 1) / 5
      keep:       =endowment + r * endowment * pg.others.contribute / 5
```

- **`<game>.others.<strategy>`** is how many of the *other* players play that strategy in the cell being worked out. It can be read only inside that game's own payoffs. It is not a scalar of the model, and a formula elsewhere that names it is refused with that reason. The name sits under the game's own name, as every result does (ADR-068, section 3), so it cannot collide with a group of the model. `others` becomes a reserved name, so no strategy, and no future result, may be called `others`.
- **The shape tells the two forms apart.** Under `payoffs:`, a strategy that maps to a number or a formula is the count form. A strategy that maps to a mapping is the table form of ADR-068. A symmetric game of two players may use either. A symmetric game of three or more players must use the count form, and the table form there is refused with a message that shows the count form.
- **Forge expands it into the full table** at the point where it reads the game (`src/games/model.rs`). Every profile gets each player's payoff from the formula, with the counts of that profile. Nothing after the parser changes: dominance, pure equilibria, the even mix, the results' names, `forge game`, Studio, the tornado and the other commands all see a table, as today. The formula is worked out once for each own strategy and each way of splitting the others among the strategies: 2 × 5 = 10 times for this game, not 32 × 5.
- **The limit stays 10,000 profiles** (`MOST_CELLS`). That allows 13 players with two strategies, or 6 players with four.
- **Results are per player, as now.** `pg.equilibrium_1.a.keep` is 1 in the equilibrium where everyone keeps. A result by count, such as how many players contribute in an equilibrium, is left out. A formula can sum the players' results if it is wanted.

*Instead:* a list of payoffs indexed by the count, for two strategies (`contribute: [4, 7.2, 10.4, 13.6, 16.8]`). It needs no new name. But it works only for two strategies, a list of numbers says less than the formula it came from, and a tornado on *r* would need a formula in each entry. *Or:* name no players, `players: 5`, and number them. That is shorter, but every result would be named `player_3`, and ADR-068 names players for its results. The recommendation keeps the names.

### 2. A tournament whose round has more than two players

**Decided: left out.**

- The nine built-in strategies are defined against one other player (tit for tat answers *the* other player's last move). Against four others, each needs a rule that no outside library fixes: answer the majority, the minimum, or each player separately. Each rule is a new strategy with no outside check.
- The `axelrod` library, which is the outside check for every tournament (ADR-068, section 5), plays two players only.
- The exact method grows with the players. Two machines of three states make 9 states. Five make 3⁵ = 243, which is still exact, but no outside tool is left to compare it with.

The repeated public goods game can be written today as a game played once whose payoffs are already the totals over the rounds, for strategies the author defines. That is how most papers report it. A tournament of many players waits for a model that needs it and an outside tool to check it.

### 3. Strategies that spread: a `dynamics:` key on a symmetric game

**Decided: a `dynamics:` key, with the discrete replicator equation.**

An equilibrium says which states are stable. It does not say which one a population reaches, or from where. In the stag hunt both all-stag and all-hare are equilibria, and which one a population ends in depends on whether its share of stag players starts above 3/4. Nothing Forge reports says that today. It is the question "strategies that spread" asks, and kinship-protocol asks it of five AIs.

The first recommendation was a table in the time-series model with `PREV`, and no new key. It was set aside because the time series needs each strategy's payoff against the population written as a formula. For two players that is short. For *n* players it is a sum over every way of splitting the others among the strategies, Σ C(*n*−1, *k*) *x*^*k* (1−*x*)^(*n*−1−*k*) *f*(*s*, *k*) for two strategies, and longer with more. The count form (question 1) already gives Forge *f*(*s*, *k*), so Forge can do the sum and a user should not have to.

```yaml
games:
  hawk_dove:
    players: [me, you]
    strategies: [hawk, dove]
    symmetric: true
    payoffs:
      hawk: {hawk: =(V - C) / 2, dove: =V}
      dove: {hawk: 0,            dove: =V / 2}
    dynamics:
      start: {hawk: 0.1, dove: 0.9}   # every strategy, adding up to 1
      generations: 50
      baseline: 5                     # added to every payoff in the equation
```

- **Which games:** a symmetric game, of any number of players and strategies, in the table form or the count form. Anything else is refused with the reason.
- **The equation:** generation by generation, *x*′ₛ = *x*ₛ (*f*ₛ + *b*) / (*f̄* + *b*). *f*ₛ is strategy *s*'s expected payoff against a population with shares *x*, over every way the other players' strategies can fall. *f̄* is the average payoff, and *b* is `baseline`, 0 if not given. It is exact arithmetic, with no step size and no solver, so no library is needed. Every *f*ₛ + *b* must be above 0, because a share cannot go below 0. A payoff that breaks this names the strategy, the generation and the baseline that would be enough.
- **`start`, `generations` and `baseline` can be formulas**, as a payoff can, so a tornado or a scenario can move them. `generations` must be a whole number from 1 to 100,000.
- **Results:** `<game>.dynamics.share.<strategy>` is the share after the last generation, and `<game>.dynamics.payoff` is the average payoff then. Only the end is a scalar. `forge game`, its `-o` output and Studio show every generation (the path), so a series never becomes thousands of scalars.
- **Left out:** continuous time (the replicator equation as a differential equation). It needs an ODE solver. `ode_solvers` is the mature crate, but it brings `nalgebra` and `simba`, which Forge does not build today, into the browser module too. It also reaches the same rest points with the same stability as the discrete equation. Mutation and finite populations (the Moran process) are left out too, until a model asks.
- **The checks:** `egttools`, a Python library for evolutionary game theory with a C++ core and the replicator equation for games of *n* players, in `validation/games`, every generation. By hand: hawk-dove with *V* = 4 and *C* = 6 goes to a hawk share of *V*/*C* = 2/3. The stag hunt goes to all stag from a start above 3/4, and to all hare from one below.
- **Note, 2026-11-21: built, with two changes.** `src/games/dynamics.rs` follows the shares as written above, with one general sum over every split of the other players (for two strategies it is the Bernstein sum of question 4, and a test holds them equal), and `validation/games/dynamics.py` compares every generation with egttools 0.2.1, whose `replicator_equation_n_player` gives the gradient *x*ₛ (*f*ₛ − *f̄*) from its own payoff table. Two things differ from the text above. First, only a strategy that is played (its share above 0) must have *f*ₛ + *b* above 0: a share of 0 stays 0 whatever its strategy pays, and in the stag hunt with baseline 0 stag pays 4*x*, so as stag dies out its share reaches 0 and stag pays 0; the rule as written refused that run of the textbook game. The message names the strategy, the generation and a baseline that is enough in every generation, above minus the least payoff in the table. Second, each step divides by Σ *x*ⱼ (*f*ⱼ + *b*), not by *f̄* + *b*: the two are the same number while the shares add up to 1, but off the simplex a rounding grows by *b* / (*f̄* + *b*) a generation, above 1 when *f̄* < 0. The suite found it in its first run: in a seeded game of three players with *f̄* = −7.75 and *b* = 10.5, one share passed 1 in generation 23 and the run was refused at generation 32. On seeds 2, 31 and 202647, 14,420 generations each, it now agrees in every one. `forge game` prints every generation and `-o` writes them as `dynamics.path`, which forge-wasm's `game` returns to Studio too; Studio does not draw them yet.

### 4. Mixed equilibria for more than two players

**Decided: the symmetric mixed equilibria of a symmetric game with two strategies, for any number of players. The general case stays out.**

In a symmetric game with strategies *A* and *B*, a population where every player plays *A* with probability *x* is an equilibrium when *A* and *B* pay the same against it:

*g*(*x*) = Σₖ C(*n*−1, *k*) *x*^*k* (1−*x*)^(*n*−1−*k*) [*f*(*A*, *k*) − *f*(*B*, *k*)] = 0, for *k* = 0 to *n* − 1 others playing *A*.

This is the same *g* as in question 3: its roots in (0, 1) are the dynamics' rest points inside the simplex, and the sign of *g*′ at a root says whether the population returns to it. So both are built on one function.

- **Who needs it:** the volunteer's dilemma, the snowdrift game of many players, and threshold public goods (the collective-risk or climate game). In each, the mixed equilibrium is the answer, and no pure one says it.
- **The library:** the `roots` crate (0.0.8, BSD-2-Clause, no dependencies, about four million downloads). Its `find_roots_sturm` uses Sturm sequences, which count the real roots in an interval. Forge can therefore prove the list complete, which its rule against an incomplete list shown as complete (ADR-068, section 2) needs. It has had no release since 2022. For a root finder with no dependencies, that reads as stable.
- **A degenerate game:** if *g* is 0 everywhere (the public goods game at *r* = *n*), every mix is an equilibrium. Forge reports the game as degenerate, names the tie, and lists none of the continuum.
- **Names:** the symmetric mixed equilibria follow the pure ones in `<game>.equilibrium_<k>`, by increasing *x*, with the same probabilities for every player. For a symmetric game of two strategies, of any number of players, `<game>.equilibrium_<k>.stable` is 1 when the dynamics return to that symmetric equilibrium after a small change in the shares, and 0 when they leave it. For other equilibria, and other games, the name does not exist, and a formula that reads it is told why.
- **The checks:** the volunteer's dilemma closed form, where each player does not volunteer with probability (*c*/*b*)^(1/(*n*−1)), for *n* from 2 to 8. Gambit's `enumpoly_solve` on seeded symmetric games of three to five players with two strategies, keeping its symmetric equilibria. And for two players, Forge's existing support enumeration, which must find the same symmetric ones.
- **Note, 2026-11-21: `polycool`, not `roots`.** Measured before building step 4, `roots` 0.0.8 does not do what this section says. Its `find_roots_sturm` does not build Sturm sequences: it brackets roots between the roots of the derivative, found the same way, and narrows each bracket, and its own test (`find_roots_sturm_tim_lueke`) records a quartic whose two real roots it cannot find. On polynomials of this shape, g with integer dₖ from −5 to 5, it missed at least one root of 41,413 of 119,818: none for three or four players (degree 2 and 3, solved by formula), and 20% to 70% of the polynomials for five to eight. `polycool` 0.4.0 (linebender, the root finder of `kurbo`; MIT or Apache-2.0; brings `arrayvec` only; 23 million downloads; released 2025-10) brackets the same way and says so, and that it can miss two roots very close together. On 9,000 of these polynomials, against sympy's exact real-root isolation, it missed one root in one polynomial, a double root (g touched 0 without changing sign), and found extra ones in 13, each within 1e-8 of 0 or 1 where g has a root of higher multiplicity at the end. Forge adds three steps around it, each from the exact algebra: a root at either end is divided out first, in Bernstein form; a point where g touches 0 is found as a root of g′ where g is within the tie; two roots between which g stays within the tie are one. With them, on 31,500 polynomials of two to eight players, every list had the same number of roots as the exact one, and every root was within 1e-7 of its exact value except three of multiplicity four or more, within 2e-5 (such a root can only be located to about the fourth root of the precision). `validation/games/symmetric.py` checks seeded games against the exact roots in CI, and against Gambit's `enumpoly_solve`, which finds fewer: it missed 2 of 36 in the first run, both near x = 1, that the exact roots confirm. "Sturm sequences" above should be read as "a complete list, checked against the exact roots".
- **Still left out: the general case.** Asymmetric mixed equilibria of three or more players, and symmetric ones with three or more strategies, are solutions of systems of polynomials. No method is both complete and simple. Gambit is the reference, and it is C++ that cannot run in the browser module. The one Rust crate that offers it, `game-theory-core`, is version 0.1.0, three months old, with 64 downloads, short of "stable, well-maintained" (CLAUDE.md). The report keeps saying what it did not look for.

## What is built, in order

1. **The count form** (question 1), in `src/games/model.rs` and the 5.5.0 schema. The format version stays 5.5.0: a symmetric game of three or more players is refused today, so no model that works now reads differently (ADR-014, ADR-067). Tests: the expanded table equals the 32-cell table written out; the five scenarios give 1, 1, 32, 1 and 1 equilibria (the hand calculation above); `others` read outside its game, a symmetric game of three players in the table form and a strategy named `others` are each refused with their place.
2. **`examples/games/public-goods.yaml`**, five players, with scenarios for *r* below, at and above *n*, and a test that asserts the equilibria and the even-mix payoffs from the formulas above.
3. **`validation/games`**: the public goods game and seeded symmetric games of three to five players with two or three strategies, in the count form, compared with Gambit's `enumpure_solve` on a table the suite expands itself.
4. **The population function and its equilibria** (question 4): *g*, its roots by `polycool` (`src/games/population.rs`), the stability, the degenerate case, the volunteer's dilemma example, and the checks above.
5. **The `dynamics:` key** (question 3): the equation, its results, `forge game` and its output, Studio's view of the path, the hawk-dove and stag-hunt examples, and `egttools` in `validation/games`. *Built 2026-11-21* (`examples/games/hawk-dove.yaml`, and dynamics in `examples/games/stag-hunt.yaml`), except Studio's view of the path, which waits for Studio's view of games of many players.
6. **Docs, each step:** docs/FEATURES.md, the game section of docs/cli, the CHANGELOG, the roadmap, and a dated note in ADR-068's "What is left out".

## Consequences

- A symmetric game of many players is written in two lines of payoffs, not 2ⁿ cells. Every command that works on a game works on it, because it is the same table by the time anything reads it.
- New names, fixed once released (docs/STABILITY.md): `others` is reserved in a game, `<game>.others.<strategy>` exists inside its payoffs, and `dynamics`, `start`, `generations`, `baseline`, `<game>.dynamics.share.<strategy>`, `<game>.dynamics.payoff` and `<game>.equilibrium_<k>.stable` are new.
- One new dependency, `polycool`, which brings `arrayvec` (`roots`, chosen first, misses roots: see question 4).
- The solver of ADR-068 does not change. The expansion is checked cell by cell against a table written out. The new equilibria and the dynamics are checked against Gambit, `egttools` and closed forms, each in CI.
- Tournaments of many players, mixed equilibria of the general case, continuous time, mutation and finite populations stay out, each with its reason and what would change it.
