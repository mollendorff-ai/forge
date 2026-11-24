# ADR-079: The Summary of a Simulation of Huge Values Is a Number

**Status:** Proposed (2026-11-24), built with it; accepted by merging it
**Date:** 2026-11-24
**Decided by:** the maintainer, who asked for the item to be built (2026-11-24) and accepts this by merging it; the analysis is Claude's, in a session configured for Opus 5.5
**Closes:** roadmap T1-HUGE-STATS

---

## Context

T1-NONFINITE (rc.30) made Forge refuse a distribution whose parameters are not finite numbers. It left one gap, found while fixing it: a distribution that is finite but wide, `MC.Triangular(-1E+307, 0, 1E+307)`, drew finite values, and `forge simulate` printed them with `Std Dev: inf` and exit 0. `Statistics::from_samples` squared each deviation from the mean, and the square of anything above about 1.3e154 is past the largest `f64` (1.8e308). With an output ten times the draw (`=x * 10`, values near 9e307), the sum of 200 of them overflowed too, and the mean was `inf`. So was every mean worked out the same way: the expected shortfall, each input's mean in `forge simulate`'s printout and in its JSON, where serde writes an infinity as `null`. A histogram from below -9e307 to above 9e307 had a width of `inf` and put every sample in its first bin.

Every one of these figures is a number: the mean lies between the smallest and the largest sample, and the standard deviation is at most the largest distance from the mean. Only the arithmetic used to reach them overflowed. The variance is different: the variance of values near 1e307 is near 1e613, which no `f64` can hold.

A model rarely draws numbers this large. But a summary that prints `inf` for a finite answer is a wrong answer, and an agent reading the JSON sees `null` where a number should be.

## Options

1. **Refuse an output whose values cannot be summarized**, with exit 1 and a message. Simple, but the values can be summarized: refusing them refuses an answer Forge can give.
2. **Work the figures out on scaled values**: divide every sample by a power of two near the largest, work out the mean and the spread, and multiply back. Dividing by a power of two is exact (it changes only the exponent), so the scaled figures are the true figures to the last digit the arithmetic allows.
3. **Scaled always**: the same, for every model. Simpler code, but the last digit of a standard deviation could change in models that summarize today, and every figure in the docs and the tests would have to be checked again.

## Decision

Option 2, falling back from the plain arithmetic:

1. **Every figure is worked out the plain way first.** Only when the result is not a number although every sample is one (the arithmetic overflowed) is it worked out again on the samples divided by the power of two at or below the largest magnitude, and scaled back. A model that summarizes today keeps every digit: `figures_that_fit_are_the_plain_formulas_to_the_last_digit` holds the mean, variance, standard deviation, skewness and kurtosis to the old formulas bit for bit, on values near 3e151.
2. **The mean** (`monte_carlo::statistics::mean`) is the one used by the statistics, the expected shortfall and each input's mean, in the printout and the JSON. It is a number whenever the samples are.
3. **The standard deviation, skewness and kurtosis** are worked out on deviations in the same scaled units, so they are numbers whenever the samples are.
4. **The variance is `None` when it is larger than the largest `f64`** (`Statistics::variance` is now an `Option<f64>`). Forge prints and answers the standard deviation, not the variance, so nothing a person or an agent reads changes; a program using the library sees that the variance has no `f64`, not an infinity.
5. **A histogram wider than the largest `f64` is cut in halves**: its ends and each sample are halved (exact), the bins cut, and the edges doubled back. A range that fits is cut exactly as before.
6. **A sample that is not a number is not scaled away.** NaN or an infinity in the samples gives what it gave before: scaling cannot make it a number.
7. **The spreadsheet functions are not changed.** `=STDEV(-1E+307, 1E+307)` and `=AVERAGE(1E+308, 1.5E+308)` are `#NUM!`, "result is too large", as in Excel, whose functions Forge's follow. A simulation's summary is Forge's own, with no spreadsheet to agree with.

## Consequences

- `forge simulate` on `MC.Triangular(-1E+307, 0, 1E+307)` prints no `inf` and no `NaN`: the standard deviation of `=x * 2` is within 2% of 2e307 / √6, as the triangle's is, and the mean of `=x * 10` is five times that of `=x * 2`. `forge_simulate` over MCP answers it with no `null` (`src/monte_carlo/distributions.rs`, `refused_everywhere`).
- The fallback costs one more pass over the samples, and only for an output whose plain arithmetic overflowed.
- `Statistics::variance` is an `Option<f64>`: a change to the library's API, in a release candidate.
