# ADR-076: What Forge Studio Shows First, and How It Helps

**Status:** Accepted (2026-11-22); built: steps 1 to 4 (2026-11-22). Step 5, the help, is next
**Date:** 2026-11-22
**Decided by:** the maintainer (2026-11-22), who found that the bagel's simulation and games are reached only through Files and that the page links to no documentation, and who chose the readers: people who use GitHub. The analysis and the measurements are Claude's
**For:** roadmap S-SETS, S-ANALYSES and S-HELP. It extends ADR-056 and its 2026-11-14 addendum (the welcome card)

---

## Context

As of 10.0.0-rc.33, at 390 px:

- **The bagel opens on `bagel.yaml`.** Its other ten files are reached through Files, "this model's files". They are listed by file name, each marked "Another model", with nothing to say that one is a simulation, two are games, and four are a decision tree, a real option, a bootstrap and a Bayesian network.
- **Four of those files open on an empty Model view,** with no message: `second_shop.yaml`, `wait_option.yaml`, `first_month.yaml` and `slow_week.yaml`. Forge's `model` answer says nothing of their analyses (its scalars, tables and games are all empty). Their tools run only on the Tools tab, and the answer is shown as JSON.
- **The only help is the welcome card's three tips** (change an input, open a formula, look around). The page links to no documentation.
- **The Tools tab cuts an answer at 6,000 characters.** The bagel price war's answer is cut.
- **The documentation has dead ends:**
  - GLOSSARY.md is linked but does not exist.
  - EXAMPLES.md tells you to use `MC.Correlated`, which is not a function.
  - FEATURES.md shows `_forge_version` 8.x, which Forge does not read.
  - Goal seek and break-even have no section in FEATURES.md.
  - `dynamics:` and `format:` have no heading of their own.

  A link from Studio needs a heading to land on.

The readers are people who use GitHub (the maintainer, 2026-11-22): they read Markdown on GitHub, follow links into a repository, and read YAML.

Measured: the six main documents (README, FEATURES, EXAMPLES, FUNCTIONS, cli/README, MCP) are 77 KB gzipped together. Studio's WebAssembly module is 1,997,400 bytes gzipped, under ADR-070's ceiling of 3,000,000.

## Decision

### 1. A set shows its models as questions

Each example set with more than one model gives each model a question, in `studio/build.mjs` where the set's title and blurb are written. For the bagel:

| File | Question |
| --- | --- |
| `bagel.yaml` | Will the first year make money? |
| `bagel_simulate.yaml` | How sure can we be? |
| `price_war.yaml` | Cut the price if the rival does? |
| `street_fund.yaml` | Will the street pay for a festival? |
| `second_shop.yaml` | Open a second shop, or test first? |
| `wait_option.yaml` | What is waiting a year worth? |
| … | … |

Each question carries a badge for its kind: plan, simulation, game, decision tree, option, bootstrap or Bayes. The page does not guess the kind: Forge's `model` answers which analyses a file holds. That is engine work: `analyses`, the analysis sections the file has, in its order, among `monte_carlo`, `tornado`, `decision_tree`, `real_options`, `bootstrap` and `bayesian_network`. Games and scenarios are listed already.

The questions appear in two places:

- **The welcome card** on a first visit becomes the set's questions, a tap on one opening it. It keeps one line on what the page is and one tip (drag an input, tap Edit on a formula), and "Got it". `studio/tests/welcome.mjs` is changed to match.
- **A strip of chips at the top of the Model view**, with the set's models by their short names and badges. The price war is then one tap from the plan, at any time. It is not put in the header, which stays on screen on every view: a second row there would take height from every view on a phone.

Files keeps its list of the files, and each "Another model" line gets its question and badge.

### 2. Every model has a Model view

A file whose analysis is a decision tree, a real option, a bootstrap or a Bayesian network opens on a card that runs its tool at once and draws the answer, not JSON:

- **Decision tree:** the best path and each choice's expected value.
- **Real option:** each option's value beside the value without it.
- **Bootstrap:** the estimate and its interval.
- **Bayesian network:** each variable's probabilities, with evidence set by tapping a state.

Each card says in a sentence what the analysis answers, and has a "?" (decision 3). The numbers are held against the tool's answer in the screen test's way. Editing these models' own sections on cards is not part of this ADR.

### 3. A "?" on each section, and help where you type

- **Each section heading has a "?"**: scenarios, inputs, results, tables, games, a tournament, dynamics, each of Analyze's tools, each kind of model in decision 2, and an uncertain model (ADR-075). It opens a sheet with:
  - two or three sentences;
  - the terms the section uses;
  - the examples that show it, which open on a tap;
  - a link to the manual (decision 4).

  The text is Studio's own, written for the page, in `studio/help.json`. It is not shown when there is no connection, apart from the link.
- **In the formula editor,** a function's suggestion shows its syntax and its one-line description from Forge's function list (`forge functions`), which Studio already loads for suggestions.
- **In the YAML editor and the Add form,** a key's suggestion shows its description from Forge's schema (112 descriptions in 5.5.0).

### 4. The manual is linked on GitHub, at the release

A "Read more" link opens the document on GitHub at the tag of the release that is running, for example `https://github.com/mollendorff-ai/forge/blob/v10.0.0-rc.33/docs/FEATURES.md#scenario-analysis`. So the text matches the engine on screen. The status sheet and Files also link to the manual and to the open example's folder at that tag.

Why not bundled into Studio: at 77 KB gzipped, size is not the reason. The reasons are:

- the documents link to source files, ADRs and examples by relative path, which work only on GitHub;
- they would need a Markdown library in the page;
- the readers are on GitHub already.

To be decided again if someone asks to read the manual offline.

A test (`studio/tests/help.mjs`) checks that every link in `help.json` and every link the page makes names a document that exists in the repository and a heading that is in it, by GitHub's rule for anchors. A heading renamed in the docs then fails the build, not a reader.

### 5. The documentation gets the headings the links need

Before the links, the dead ends listed in Context are fixed:

- a GLOSSARY.md;
- `MC.Correlated` replaced by `correlations:`;
- FEATURES.md's `_forge_version` values corrected;
- a FEATURES.md section for goal seek and break-even;
- headings for `dynamics:` and `format:`.

docs/architecture/README.md is corrected too: ADR-055 and ADR-056 are built.

### 6. Tools shows the whole answer

An answer longer than a screen is shown in a box that scrolls, with "Copy" and "Download" (JSON), and is never cut.

### 7. Not now: a guided tour

The tour parked on 2026-11-13 stays parked. The questions in decision 1 and the help in decision 3 do part of its work. Whether a tour is needed is for T2-FIRST-USER to show.

## Consequences

- The first screen says what a set can do. A simulation, a game and a decision are one tap from the plan.
- No model opens on an empty page.
- A newcomer can always reach the right paragraph of the manual, for the release they are running.
- `model` grows by one field. The page grows by the help text: a few KB.

## What is built, in order

1. The documentation's headings and dead ends (decision 5).
2. Tools shows the whole answer (decision 6).
3. `model` answers `analyses`. The questions, badges and strip (decision 1).
4. The Model view of the four other analyses (decision 2).
5. The "?" sheets, the function and key help, the links, and `studio/tests/help.mjs` (decisions 3 and 4).
