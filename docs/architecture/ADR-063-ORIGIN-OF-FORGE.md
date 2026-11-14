# ADR-063: Where Forge Came From, and Why It Is Not a Trading Engine

**Status:** Proposed (2026-11-14); accepted by merging this pull request
**Date:** 2026-11-14
**Decided by:** the maintainer (the account below is his, stated on 2026-11-14; accepted by merging this ADR); the text is Claude's, in a session configured for Sonnet 5.5
**Replaces:** the reasoning of ADR-026 (FPGA acceleration for high-frequency trading), deleted on 2026-11-14 (T1-ADR-PRUNE)

---

## Context

Forge's history is public, and parts of it look like another project: a licence that closed the source, a go-to-market plan, and an ADR about FPGAs for high-frequency trading. None of it explains why the project became what it is today. The reasons were never written down. This ADR does that, so that the history reads as a decision and not as a gap.

What the repository shows, by date:

| Date | What the history says |
| --- | --- |
| 2025-11-23 | First commit, `mouvify-forge` v0.1.0: a Rust calculator for YAML files with embedded formulas, under the Elastic License 2.0 |
| 2025-12-04 | v5.0.0: a proprietary licence |
| 2025-12-08 | The source is closed (ADR-011) |
| 2025-12-17 | ADR-026: FPGA acceleration for high-frequency trading, deferred "until trading AUM > $1M" |
| 2025-12-29 | Elastic License 2.0 again, with a go-to-market plan (ADR-030, since deleted) |
| 2026-02-16 | Relicensed MIT OR Apache-2.0 and opened as a portfolio project (ADR-031) |

## The account

As the maintainer tells it:

- **The idea.** A high-frequency trading engine was planted by the film *The Hummingbird Project*, before the pandemic.
- **The language.** He tried D and Rust. Both are good; Rust's community grew and D's did not. In 2025 he decided to build the trading engine in Rust.
- **The format.** YAML was chosen for the models because language models have been trained on a great deal of it, so that they can write a model without inventing the numbers. The trading engine was to use it.
- **The proprietary period.** The project was closed source for a few months, on a self-hosted Git server, built on the maintainer's own machines, with no client and no income.
- **The turn.** Building the YAML calculator showed that it was valuable by itself. High-frequency trading is speed at any energy cost, the opposite of green coding, which the maintainer practises and has taught. So the effort moved to financial modelling, and the trading engine was never built.

## Decision

Forge is a financial modelling engine for FP&A and for AI agents. It is not a trading engine, and FPGA or high-frequency work is not planned.

Two reasons, one written then and one written now:

1. **Cost and need (ADR-026, 2025-12-17).** Forge's performance (about 96,000 rows a second) was no bottleneck for FP&A, and hardware acceleration needed $100,000 to $500,000 and a funded trading business first.
2. **Green coding (the maintainer, 2026-11-14).** Speed for its own sake works against the energy efficiency that was a reason for choosing Rust. The date of the turn was not recorded: ADR-026 still deferred the work in December 2025, and by February 2026 the project was being opened.

## Consequences

- The history stays as it is. It is not rewritten, and the proprietary period is not hidden: ADR-031 already records the licence change.
- ADR-026 stays deleted; this ADR holds its reasoning.
- The README and the posts can say where Forge came from, and why it is open, in a sentence.
- The performance ladder of ADR-026 (algorithmic work, SIMD, threads, GPU) is not on the roadmap and is not promised.
