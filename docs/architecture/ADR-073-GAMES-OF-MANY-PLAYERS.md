# ADR-073: Games of Many Players, and Strategies That Spread

**Status:** Proposed. Four questions are for the maintainer (below); nothing is built
**Date:** 2026-11-21
**Decided by:** the maintainer, once the four questions are answered; the analysis and the recommendations are Claude's
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

## The four questions

### 1. How a symmetric game of more than two players is written

**Recommended: a payoff for each of a player's own strategies, written as a formula over how many of the other players play each strategy.**

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

**Recommended: left out.**

- The nine built-in strategies are defined against one other player (tit for tat answers *the* other player's last move). Against four others, each needs a rule that no outside library fixes: answer the majority, the minimum, or each player separately. Each rule is a new strategy with no outside check.
- The `axelrod` library, which is the outside check for every tournament (ADR-068, section 5), plays two players only.
- The exact method grows with the players. Two machines of three states make 9 states. Five make 3⁵ = 243, which is still exact, but no outside tool is left to compare it with.

The repeated public goods game can be written today as a game played once whose payoffs are already the totals over the rounds, for strategies the author defines. That is how most papers report it. A tournament of many players waits for a model that needs it and an outside tool to check it.

### 3. Replicator dynamics: in a game section, or in the time-series model?

**Recommended: in the time-series model, as an example, with no new section.**

A strategy's share in generation *t* + 1 is its share in *t*, times its payoff against the population, over the average payoff (the discrete replicator equation). That is a row of a table that reads the row above, which is what `PREV` does (`=PREV(share * payoff / average, 0.5)`), and what `predator_prey.yaml` already does for populations. For a symmetric game of two strategies, the payoff against a population with share *x* is a short formula over the game's payoff cells, written as scalars that the game and the table both read.

- **For:** no new section, result or calculation in the engine. The numbers are a table the model already knows how to calculate, export, chart in Studio and diff in git. A user sees every step of the dynamics as formulas.
- **Against:** with many strategies or many players, the payoff against a population is a long formula, a sum over the ways of splitting the others, written by hand.
- **The check:** an example in `examples/games` of the hawk-dove game (prize *V*, cost of a fight *C* > *V*), whose shares converge to the mixed equilibrium. The hand calculation is that hawk's share goes to *V*/*C*. Hawk against hawk pays (*V* − *C*)/2, which is negative, and the discrete equation needs positive payoffs, so the example adds a baseline fitness to every payoff. That moves the speed of convergence, not where it ends. A test compares 50 generations with the same recurrence written in Python.

*Instead:* a `dynamics:` key on a symmetric game (`generations: 50`, `start: {hawk: 0.1}`), with results `<game>.share.<generation>.<strategy>`. That handles many strategies with no formula to write, but it is a new kind of result, the first one that is a series, and each choice in it (discrete or continuous time, payoffs that can be negative) is a format decision kept for all of 10.x. Recommended only if the example shows the formulas cannot be written for a model someone needs.

### 4. Mixed equilibria for more than two players

**Recommended: still left out, as in ADR-068.**

- The public goods game does not need them: one strategy dominates unless *r* = *n*.
- In general they are the roots of polynomial systems, with no method that is both complete and simple. Gambit's `enumpoly` exists but is not one Forge could copy and check in a few hundred lines.
- One narrow case has a closed form and a known use: the **symmetric** mixed equilibrium of a symmetric game with **two** strategies is a root of one polynomial in one share. The volunteer's dilemma, where one volunteer of *n* is enough, has it in closed form: each player does not volunteer with probability (*c*/*b*)^(1/(*n*−1)). If a model needs it, this case is the one to add, checked against that formula and Gambit's `enumpoly_solve`. It needs its own decision.

The report keeps saying that it did not look for mixed equilibria in a game of three or more players.

## What would be built, if the recommendations are accepted

1. **The count form** (question 1), in `src/games/model.rs` and the 5.5.0 schema. The format version stays 5.5.0: a symmetric game of three or more players is refused today, so no model that works now reads differently (ADR-014, ADR-067). Tests: the expanded table equals the 32-cell table written by hand; the five scenarios give 1, 1, 32, 1, 1 equilibria (the hand calculation above); `others` read outside its game, a count form under a table form, a symmetric game of three players in the table form and a strategy named `others` are each refused with their place.
2. **`examples/games/public-goods.yaml`**, five players, with scenarios for *r* below, at and above *n*, and a test that asserts the equilibria and the even-mix payoffs from the formulas above.
3. **`validation/games`**: the public goods game and seeded symmetric games of three to five players with two or three strategies, written in the count form and compared with Gambit's `enumpure_solve` on the table the suite expands itself.
4. **The hawk-dove dynamics example** (question 3), with its test against Python.
5. **Docs:** docs/FEATURES.md, the game section of docs/cli, the CHANGELOG, the roadmap, and a dated note in ADR-068's "What is left out".

## Consequences

- A symmetric game of many players is written in two lines of payoffs, not 2ⁿ cells. Every command that works on a game works on it, because it is the same table by the time anything reads it.
- One name is reserved in a game, `others`, and one name is new inside a game's payoffs, `<game>.others.<strategy>`. Both are fixed once released (docs/STABILITY.md).
- The solver does not change. A bug in the expansion would give a wrong table, and the tests compare the expanded table with one written out, cell by cell, so the expansion is where a wrong answer would show.
- Tournaments of many players, mixed equilibria beyond two players (save the narrow case above) and a dynamics section stay out, each with the reason and with what would change the decision.
