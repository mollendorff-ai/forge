# ADR-050: Keep All Ten Analysis Engines Supported

**Status:** Accepted
**Date:** 2026-11-08
**Decided by:** the maintainer

---

## Context

Forge has ten analysis commands beyond calculate: scenarios, compare, simulate (Monte Carlo), sensitivity, goal-seek, break-even, tornado, variance, decision-tree, real-options, bootstrap and Bayesian networks. On 2026-11-07 the roadmap asked (T1-SCOPE) whether one maintainer should keep all of them under the STABILITY.md promises, or mark the ones no user had asked for (Bayesian, real options, decision trees, bootstrap) as experimental.

The same day, T1-USE built one real model per command (`examples/bagel/`) and checked each headline number against a calculation written outside Forge. The numbers were right in every engine. The bugs were in what surrounded them: variance favourability, output order, an oversized bootstrap reply, unreported ties. They were fixed in 10.0.0-rc.5.

## Decision

**All analysis engines stay supported, and STABILITY.md's promises cover all of them.** None is marked experimental.

The maintainer asked for these engines and uses them; scenario analysis and simulation are likely to grow.

## Consequences

- Each engine carries maintenance cost on every calculator change. What keeps that cost bounded is the T1-USE rule (roadmap T1-USE-AGAIN): every new or changed command ships with an `examples/` model whose headline number is asserted against an outside calculation.
- New scenario and simulation features are expected. They follow the same rule, and they should extend the existing `scenarios:` and `monte_carlo:` sections instead of adding parallel ones.
- If an engine later turns out to be wrong in a way a patch cannot fix, it is marked experimental then, with a CHANGELOG entry, rather than in advance.
