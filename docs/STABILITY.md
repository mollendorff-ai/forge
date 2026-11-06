# Stability

What Forge promises once it leaves beta, and what it does not. Forge is still a beta (v10.0.0-beta.11); this page is the draft of the promise, so that "stable" means something specific. Nothing here changes how the beta behaves today.

## What stable promises

1. **No silent wrong answers.** If Forge cannot compute something it returns an error, never a plausible-looking number. A wrong result in a documented function is a bug and is fixed in a patch release with a test.
2. **The model format is versioned.** A YAML model with a given `_forge_version` keeps working in every later 10.x release. A change that would break existing models needs a new major version and a documented migration.
3. **Commands and flags are kept.** The commands in the README (`validate`, `calculate`, `audit`, `export`, `import`, `sensitivity`, `goal-seek`, `break-even`, `variance`, `simulate`, `scenarios`, `decision-tree`, `real-options`, `tornado`, `bootstrap`, `bayesian`) and their documented flags are not removed or renamed within 10.x. Wording of messages and the order of lines in human-readable output may change.
4. **Same input, same output.** Results and output order are deterministic. Simulations with a `seed` give the same numbers on every run on the same version.
5. **Semantic versioning.** Patch releases fix bugs, minor releases add features, major releases may break the points above. Every release has a changelog entry.
6. **Checks stay in CI.** The unit tests run on every change. The Gnumeric and R suites run weekly and on any change to the dependencies or the workflow; a failing suite blocks a stable release.

## What stable does not promise

- **Excel itself.** Formulas are compared with Gnumeric and R, and the Excel export is recalculated by Gnumeric, not by Excel. Real Excel was checked by hand once, on 18 financial and date results (see [VALIDATION.md](../VALIDATION.md)); it is not repeated automatically.
- **Everything in the analytics is checked.** 70 of 77 analytics specs are compared with R; the exponential distribution, the elasticity command, one-point ranges and an input with equal low and high are not covered. Real options are compared on price only.
- **Numbers to the last digit.** Floating-point results can differ from other engines in the last digits; the suites use tolerances (listed in VALIDATION.md).
- **Financial, legal or tax advice.** Check results that matter independently.
- **Support.** Forge has one maintainer and no known users yet. Fixes are made as time allows.
- **Security of untrusted models.** Forge is meant for models you wrote or trust.

## What blocks the first stable release

- At least one person other than the author using a release on a real model and reporting back. One outside contributor used Forge for a few weeks and has since paused; that is evidence of use, not of current use.
- The Gnumeric and R suites green on the release commit.
- No open report of a wrong answer.
