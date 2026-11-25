# Forge documentation

Where to start, by who you are, and then every page by what it holds. [ADR-078](architecture/ADR-078-A-WAY-IN-FOR-EACH-READER.md) says why the pages are arranged this way. The front page of the project is the [main README](../README.md).

## Where to start

| You are | Start at | Then |
| --- | --- | --- |
| **New to Forge**, with or without a terminal | [The guide](guide/README.md): lessons on one model, in order | [A tour of Forge Studio](guide/studio.md); the reference when a lesson links to it |
| **Working with an AI**, or building an agent | [Forge with an AI](guide/ai.md) | [Prompts for your AI](PROMPTS.md); [the MCP reference](MCP.md); [the agent skill](../skills/forge-modelling/SKILL.md) |
| **Building a pipeline** | [Models checked on every pull request](guide/pipeline.md) | [Workflows](WORKFLOWS.md); [the command line reference](cli/README.md) |
| **Embedding Forge** in a program or a page | [Embedding Forge](EMBEDDING.md) | [docs.rs](https://docs.rs/mollendorff-forge); [forge-wasm](../forge-wasm/README.md); the [REST API](FEATURES.md#http-rest-api-server) |
| **Changing or releasing Forge** | [Contributing](../CONTRIBUTING.md) | [AGENTS.md](../AGENTS.md), the rules; [Maintaining Forge](MAINTAINING.md); [the architecture](architecture/README.md) |

## Learn

Lessons, read in order, on one model: Bagel Lune, a made-up bagel shop.

- [The guide's index](guide/README.md)
- [1. The first ten minutes](guide/01-first-ten-minutes.md): open the shop, calculate it, change the price
- [2. A first model](guide/02-a-first-model.md): write it from an empty file
- [3. Keeping a model](guide/03-keeping-a-model.md): `forge edit`, `forge calculate`, `forge validate`, git
- [4. What if](guide/04-what-if.md): scenarios, break-even, goal seek, the tornado
- [5. How likely](guide/05-how-likely.md): uncertain inputs and `forge simulate`
- [6. What will they do](guide/06-what-will-they-do.md): a price war, as a game
- [Forge Studio, a tour](guide/studio.md): the browser app, with pictures
- [Forge with an AI](guide/ai.md): a chat that runs commands, the MCP server in a client, rules for an agent
- [Models checked on every pull request](guide/pipeline.md): `forge validate` as a gate in GitHub Actions
- [Examples](EXAMPLES.md): a longer walk through models, mostly Monte Carlo
- [The posts](posts/README.md): eight questions, each answered with one command on `examples/`
- [Prompts for your AI](PROMPTS.md): seven prompts to paste into an AI that can run commands

## Look things up

- [Features](FEATURES.md): the model file, every command, every analysis
- [Functions](FUNCTIONS.md): every function, with an example
- [The command line reference](cli/README.md): every command's `--help`
- [The MCP server](MCP.md): its tools and how to configure a client
- [Glossary](GLOSSARY.md): the terms Forge, its documents and Studio use
- [forge report](REPORT.md): models side by side
- [FP&A prediction methods](FPA-PREDICTION-METHODS.md): which technique answers which question
- [Stability](STABILITY.md): what a stable release promises, and what it does not

## Build with Forge

- [Workflows](WORKFLOWS.md): four ways to use Forge, as diagrams
- [Embedding Forge](EMBEDDING.md): the library, `forge serve`, `forge-wasm` and `forge mcp`

## Change and release Forge

- [Contributing](../CONTRIBUTING.md): set-up, and what a change needs
- [Maintaining Forge](MAINTAINING.md): how a release is made
- [The architecture](architecture/README.md): how Forge is built, and every decision (ADR) behind it
- [Validation](../VALIDATION.md): what is checked against what, with the latest results
- [The changelog](../CHANGELOG.md) and [its archive](CHANGELOG_ARCHIVE.md), before v8.0.0
- [The roadmap](../.asimov/roadmap.yaml): what is next, in order

## Every page is checked

`validation/repo`'s `docs_index` check fails if a page of `docs/` is not linked from here, if a link goes nowhere, or if the main README's documentation table does not link this page.
