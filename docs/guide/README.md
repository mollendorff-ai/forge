# The Forge guide

Lessons, read in order, on one model: Bagel Lune, a made-up bagel shop in its first year. Each lesson shows the YAML, the command and what Forge printed, and ends with a result to check another way and a question to answer yourself.

| Lesson | What you do |
| --- | --- |
| [1. The first ten minutes](01-first-ten-minutes.md) | Open the bagel shop in Studio or install Forge, calculate it, change the price and see what moved |
| [2. A first model](02-a-first-model.md) | Write the shop from an empty file: inputs, a formula, a table of months, the year's totals, a second file, scenarios |
| [3. Keeping a model](03-keeping-a-model.md) | Change it the way a model is kept: `forge edit`, `forge calculate` and its `.bak`, `forge validate` when a result is out of date, git, `forge upgrade` |

Lesson 1 calculates [`examples/bagel/`](../../examples/bagel/), the model the README is about. The model lesson 2 writes is in [`examples/guide/`](../../examples/guide/), so it runs in the tests like the others, and lesson 3 keeps it.

## Every page is checked

The tests run every `forge` command these pages show, in a copy of `examples/`, and fail if a command prints other than what the page shows, if a file the page shows differs from the one in `examples/`, if a link goes nowhere, or if an answer is a number no command printed (`validation/repo`'s `guide` module, [ADR-078](../architecture/ADR-078-A-WAY-IN-FOR-EACH-READER.md)). The output shown is the lines that matter; Forge prints a few more around them.

## Where to look things up

The lessons link to the reference rather than repeat it:

- [FEATURES.md](../FEATURES.md): the model file, every command and analysis
- [FUNCTIONS.md](../FUNCTIONS.md): every function, with an example
- [The CLI reference](../cli/README.md): every command's `--help`
- [GLOSSARY.md](../GLOSSARY.md): the terms
- [The posts](../posts/README.md): questions about the bagel shop and other models, each answered with one command
