# Forge MCP Integration Guide

Forge implements the [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) over stdin/stdout using JSON-RPC 2.0. This gives AI agents structured access to all 22 Forge tools.

Forge uses [rmcp](https://github.com/modelcontextprotocol/rust-sdk) (the Model Context Protocol project's official Rust SDK) for protocol handling. Tool definitions use typed request structs with automatic JSON schema generation. Additional transports (SSE, WebSocket) are planned for a future release.

## Quick Start

### Claude Desktop

Open Settings, Developer, Edit Config in Claude Desktop, or edit the file it names: `~/Library/Application Support/Claude/claude_desktop_config.json` on macOS, `%APPDATA%\Claude\claude_desktop_config.json` on Windows. Add:

```json
{
  "mcpServers": {
    "forge": {
      "command": "forge",
      "args": ["mcp"]
    }
  }
}
```

### Claude Code

```bash
claude mcp add forge -- forge mcp
```

### Any MCP Host

Launch the server:

```bash
forge mcp
```

The server reads JSON-RPC requests from stdin and writes responses to stdout, one JSON object per line.

## Keeping Files Inside One Folder

The tools read and write the files an agent names, with your permissions: `forge_calculate` writes results into the model, `forge_export` and `forge_import` write a workbook or a model wherever `excel_path` or `yaml_path` points. That is normal for a local MCP server, but it means an agent, or a prompt injected into one by a document it read, can read any model or workbook you can and write a file anywhere you can.

`forge mcp` limits every path to one folder: the folder the server starts in, or the one given with `--root DIR`. Name it in a desktop app's configuration, which may start the server in `/` or your home folder:

```json
{
  "mcpServers": {
    "forge": {
      "command": "forge",
      "args": ["mcp", "--root", "/Users/you/models"]
    }
  }
}
```

- A path is resolved first, `..` and symbolic links followed as the system would, and refused unless the result is inside `DIR`. This holds for reading (`file_path`, `yaml_path`, `budget_path`, `actual_path`, an import's `excel_path`) and for writing (an export's `excel_path`, an import's `yaml_path`, a calculation written back to its model).
- A model that includes a file outside `DIR`, directly or through another include, is refused before the included file is opened.
- A link that points to a file that does not exist yet is refused, as writing it would create that file wherever the link points.
- A relative path is relative to `DIR`, and the server's instructions tell the agent the folder.
- Inline `content` is unchanged: it is written to a temporary folder, and its includes must already be plain relative paths.
- `forge mcp --any-path` lifts the limit: any path you can reach, as before rc.28. `forge serve` has no limit unless given `--root DIR`.

The check runs before each command does, so it limits what requests name; it is not a sandbox. A process that can change links inside `DIR` while a request runs could get around it.

## Inline Content Mode

MCP clients running in sandboxed environments (Claude.ai, Cursor, containers) cannot pass a `file_path` because the path does not exist on the Forge host. These clients can pass YAML content directly using the `content` parameter instead.

All 16 file-path tools accept `content` as an alternative to `file_path`. The two are mutually exclusive -- provide one or the other, never both.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_validate","arguments":{"content":"_forge_version: \"5.1.0\"\nassumptions:\n  price: 100\n  cost: 60\n  profit:\n    formula: \"=assumptions.price - assumptions.cost\"\n"}}}
```

The server writes the content to a temporary directory, passes the temp path to the calculation engine, and cleans up automatically when the request completes.

### Inline Includes

A model can read scalars from other YAML files: list them in `_includes` and write `@namespace.name` in a formula (`@rates.fx.usd_eur`; see the Values from Other Files section of [FEATURES.md](FEATURES.md)). In inline mode, pass each included file's content through the `includes` parameter -- a map of namespace names to YAML content strings. The namespace key must match the `as` field in the model's `_includes` entries. In this example `sales.amount_eur` calculates to 920.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_calculate","arguments":{"content":"_forge_version: \"5.1.0\"\n_includes:\n  - file: rates.yaml\n    as: rates\nsales:\n  amount: 1000\n  amount_eur:\n    formula: \"=sales.amount * @rates.fx.usd_eur\"\n","includes":{"rates":"_forge_version: \"5.1.0\"\nfx:\n  usd_eur: 0.92\n"},"dry_run":true}}}
```

An included file can include others: `examples/bagel/bagel_simulate.yaml` includes `costs_risk.yaml` (as `costs`), which includes `costs.yaml` (as `base`). Pass the content of every file in the chain in the same `includes` map, each under its own `as` name (`{"costs": "<costs_risk.yaml>", "base": "<costs.yaml>"}`); a nested file's `file` is relative to the file that names it, as on disk. A namespace is one file: the same name for two different files is refused, and the same file named twice is written once. The tools then give the numbers they give for the files (the tests in `src/mcp/server/` run `examples/bagel` both ways and assert the same results).

If the model YAML contains `_includes` entries but no `includes` map is provided, the server returns a clear error listing the missing namespaces; so does an included file whose own includes have no content in the map.

### Write-Back with Inline Content

When calling `forge_calculate` with `content` and `dry_run: false`, the server cannot update a persistent file (there is none). Instead, the response includes a `calculated_content` field containing the full YAML with computed values filled in. The client can use this content for subsequent calls.

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"forge_calculate","arguments":{"content":"_forge_version: \"5.1.0\"\nassumptions:\n  price: 100\n  cost: 60\n  profit:\n    formula: \"=assumptions.price - assumptions.cost\"\n","dry_run":false}}}
```

The response will contain both the calculation results and a `calculated_content` field with the updated YAML.

## Available Tools (22)

| Category | Count | Tools |
|----------|-------|-------|
| **Core** | 6 | validate, calculate, edit, audit, export, import |
| **Analysis** | 5 | sensitivity, goal_seek, break_even, variance, compare |
| **Engines** | 8 | simulate, scenarios, decision_tree, real_options, tornado, bootstrap, bayesian, game |
| **Discovery** | 3 | schema, functions, examples |

## Tool Reference

### Core Tools

#### `forge_validate`

Validate a Forge YAML model file for formula errors, circular dependencies, and type mismatches, and check each stored value against its formula. When the model states its results (`expect:`, format 5.6.0, ADR-074), it calculates the file and each scenario that has one: `expectations_checked` counts them, `expectations_met` is false when one differs, and `expectation_differences` lists each with its `scenario` (null for the file as written), `name`, `expected`, `formula` when it was written as one, `got` and `diff`, or a `reason` when the model gives no number by that name. A difference is data, as a stale value is, not a tool error.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to the YAML model file to validate |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map (use with `content`) |
| `verbose` | boolean | no | Show verbose output (default: false) |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_validate","arguments":{"file_path":"/path/to/model.yaml"}}}
```

#### `forge_calculate`

Calculate all formulas in a Forge YAML model and optionally update the file.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to the YAML model file |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `dry_run` | boolean | no | Perform a dry run without updating file (default: false) |
| `scenario` | string | no | Scenario name to apply |
| `plan` | boolean | no | Calculate an uncertain model at plan: each `MC.*` input at its plan figure, as `forge_simulate` does for `at_plan`; the answer's `at_plan` lists the inputs and their figures, and nothing is written (default: false, which refuses an uncertain model) |

*One of `file_path` or `content` is required. When using `content` with `dry_run: false`, the response includes a `calculated_content` field. Under a `scenario` nothing is written, and there is no `calculated_content`: a scenario's results are not the file's (ADR-074). When the run has an `expect:` (the file's, or the scenario's), `expectations` lists each, checked as `forge_validate` checks it, with its full name (`path`) and `met`.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_calculate","arguments":{"file_path":"/path/to/model.yaml","dry_run":true}}}
```

#### `forge_edit`

Change a model's text, comments and layout kept (ADR-075): set an input's value or a formula, add an input, a formula or a table's column, delete one, rename a name and every name that reads it, make an input uncertain and certain again, or add or delete a game's strategy or player, in the model or in a file it includes. Every byte outside a change stays as it was. A change is made only when Forge reads the model back valid, with that change and nothing else; when one of the changes cannot be made, none is, and the message names it and says why. The answer is `files`, the new text of each file that changed by its path from the model's folder (`model.yaml`, `data/costs.yaml`), and `written`. With `file_path` the files are written, each kept as it was in a `.bak` next to it, unless `dry_run`; with `content` nothing is written (the main file is `model.yaml`). Results are not written: `forge_calculate` does that.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to the YAML model file |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `changes` | array | yes | The changes, made in turn (below) |
| `dry_run` | boolean | no | Answer the new text without writing the files (default: false) |

*One of `file_path` or `content` is required.

Each change has an `op` and a `path`, the name as formulas read it (`price`, `year.price`, a table's column `months.sales`, `@costs.rent` in the file included as `costs`):

| `op` | Takes | Does |
|------|-------|------|
| `set_value` | `value` | An input's value: a number, or its text as YAML holds it (`1e-5`, a date `2026-01-31`). The rest of the line, a comment or the rest of a flow mapping, stays |
| `set_formula` | `formula` | A scalar's or a column's formula, one line starting with `=`, in the quotes the file already uses for it |
| `add` | `value` or `formula`, and `unit`; `after` at the top level | An input or a formula at the end of its group, in the indentation of the keys there; a formula in a table is a column. At the top level it goes after the block of the name `after` |
| `delete` | | An input, a formula or a column, with the lines under it and its comment on the line; a comment above it stays. The last name of a group is not deleted |
| `rename` | `new_name` | A scalar, a group, a table, a column, a game, a player or a strategy called `new_name`, one word (`cost` for `year.price`), and every name that reads it, in every file of the model: formulas (a game's payoffs and a Bayesian network's priors and tables too), scenarios and their expectations, `expect:`, a simulation's outputs and correlated inputs, a tornado's output and inputs. What a name reads is found as the calculator reads it: a column's bare name reads its table first, a scalar's short name its group first. Each is written again as it was, full or short; a short name that would read something else once renamed is written in full. `@costs.rent` is renamed in the included file and where the model reads it. A game is `price_war`: its key, a tournament's `round:` and its results (`price_war.equilibrium_1.lune.payoff`). A player is `price_war.players.lune`, a tournament's `rivalry.players.steady`: its place in `players:` (and `strategies:`) and the results under it (`price_war.dominated.lune.hold`, `rivalry.score.steady`). A strategy is `price_war.strategies.hold`, every player's of that name, or `inspection.strategies.human.audit`, one player's in a game that is not symmetric: its lists, each level of `payoffs:` that is its player's, `dynamics.start`, and the results that end with it (`price_war.dynamics.share.hold`, `pg.others.contribute`). Forge then reads the model back: every name reads what it read, renamed, and nothing else changed. Players written once and read by two tournaments (`players: *roster`) are refused |
| `make_uncertain` | `formula` | An input (a number, not a date) becomes a distribution with its value kept as the plan: `value:` and `formula:` under its name, in the style the file uses for it (`price: 16` becomes a block, a block gets a `formula:` line, a flow mapping gets `formula:` inside). `formula` is a distribution, `=MC.PERT(12, 16, 22)`; without it, a triangle from 80% to 120% of the value, the value its mode (a value of 0 needs `formula`). A model with no `monte_carlo:` gets one at the end of the main file (`iterations: 5000`, `seed: 7`, `sampling: latin_hypercube`, and the model's results, the formulas nothing else reads, as `outputs`). Forge reads the model back at plan. An input of a file that another included file includes too is refused: that file calculates it without a plan figure |
| `make_certain` | `value` | A distribution becomes an input. Its `value:` is kept, or replaced by `value`; with none, the distribution's plan figure (its mode or mean, the midpoint of a uniform) takes the formula's place. A name left with `value:` alone is written short (`price: 16`), so `make_uncertain` and `make_certain` give back the file as it was. The `monte_carlo:` block stays |
| `add_strategy` | | A strategy of a game played once: `sealed_bids.strategies.bid_50` for every player (or the list they share), `inspection.strategies.human.ignore` for one player's in a game that is not symmetric. It goes at the end of each list, at each level of `payoffs:` that is its player's, with cells of 0 written as the cells beside it are (`[0, 0]` for each player's payoff, `0` in a symmetric game, a line of its own or inside `{...}`), in the count form (`match: 0`) and in `dynamics.start` (a share of 0) |
| `delete_strategy` | | A strategy, every player's of that name or one player's, with its cells, its payoff of the count form and its share in `dynamics.start`. A player keeps one strategy or more |
| `add_player` | `plays` for a tournament's | A player of a game played once, `price_war.players.shop`, last: its strategies are the last player's (copied when each player has a list), and each cell becomes the new player's level, the cell as it was for each of its strategies with a payoff of 0 for it (`[3, 2]` becomes `{opera: [3, 2, 0], football: [3, 2, 0]}`). In the count form only `players:` changes. A symmetric game of two written out cannot have a third: Forge says how to write it in the count form. A tournament's player, `rivalry.players.copycat`, plays the habit `plays` (`tit_for_tat`, `random`, ...) |
| `delete_player` | | A player, and its level of `payoffs:`: what is left is the payoffs of its first strategy, with its payoff out of each cell, so `add_player` and `delete_player` give back the file as it was. A game keeps two players or more, and a tournament two players or more |

A new strategy or player that the model cannot have is refused with Forge's message: a strategy of the game a tournament plays (its round has two strategies), a third player of a symmetric game written out, a strategy or player whose results a formula reads (`vd.equilibrium_6` when there are five), or a player of `players: *roster`, which two tournaments share.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_edit","arguments":{"file_path":"/path/to/model.yaml","changes":[{"op":"set_value","path":"year.price","value":17},{"op":"add","path":"year.tax","value":0.15,"unit":"ratio"},{"op":"rename","path":"year.price","new_name":"unit_price"}]}}}
```

#### `forge_audit`

Audit a specific variable to see its dependency tree and calculated value.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to the YAML model file |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `variable` | string | yes | Name of the variable to audit |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_audit","arguments":{"file_path":"/path/to/model.yaml","variable":"assumptions.profit"}}}
```

#### `forge_export`

Export a Forge YAML model to an Excel workbook.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `yaml_path` | string | no* | Path to the YAML model file |
| `content` | string | no* | Raw YAML model content (alternative to `yaml_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `excel_path` | string | yes | Path for the output Excel file |

*One of `yaml_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_export","arguments":{"yaml_path":"/path/to/model.yaml","excel_path":"/path/to/output.xlsx"}}}
```

#### `forge_import`

Import an Excel workbook into a Forge YAML model. This tool is file-based only -- inline content is not supported.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `excel_path` | string | yes | Path to the Excel file to import |
| `yaml_path` | string | yes | Path for the output YAML file |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_import","arguments":{"excel_path":"/path/to/input.xlsx","yaml_path":"/path/to/output.yaml"}}}
```

### Analysis Tools

#### `forge_sensitivity`

Run sensitivity analysis by varying one or two input variables and observing output changes.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to the YAML model file |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `vary` | string | yes | Name of the input variable to vary |
| `range` | string | yes | Range: start,end,step (e.g., `80,120,10`) |
| `output` | string | yes | Name of the output variable to observe |
| `vary2` | string | no | Second variable for 2D analysis |
| `range2` | string | no | Range for second variable |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_sensitivity","arguments":{"file_path":"/path/to/model.yaml","vary":"price","range":"80,120,10","output":"profit"}}}
```

#### `forge_goal_seek`

Find the input value needed to achieve a target output. Uses bisection solver.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to the YAML model file |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `target` | string | yes | Name of the target output variable |
| `value` | number | yes | Desired value for the target |
| `vary` | string | yes | Name of the input variable to adjust |
| `min` | number | no | Minimum bound for search |
| `max` | number | no | Maximum bound for search |
| `tolerance` | number | no | Solution tolerance (default: 0.0001) |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_goal_seek","arguments":{"file_path":"/path/to/model.yaml","target":"profit","value":100000,"vary":"price"}}}
```

#### `forge_break_even`

Find the break-even point where an output equals zero.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to the YAML model file |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `output` | string | yes | Name of the output variable to find zero crossing |
| `vary` | string | yes | Name of the input variable to adjust |
| `min` | number | no | Minimum bound for search |
| `max` | number | no | Maximum bound for search |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_break_even","arguments":{"file_path":"/path/to/model.yaml","output":"profit","vary":"units"}}}
```

#### `forge_variance`

Compare budget vs actual with variance analysis.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `budget_path` | string | no* | Path to the budget YAML file |
| `budget_content` | string | no* | Raw YAML content for the budget model (alternative to `budget_path`) |
| `actual_path` | string | no* | Path to the actual YAML file |
| `actual_content` | string | no* | Raw YAML content for the actual model (alternative to `actual_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map (shared by both models) |
| `threshold` | number | no | Variance threshold percentage for alerts (default: 10) |

*One of `budget_path` or `budget_content` is required. One of `actual_path` or `actual_content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_variance","arguments":{"budget_path":"/path/to/budget.yaml","actual_path":"/path/to/actual.yaml"}}}
```

#### `forge_compare`

Compare calculation results across multiple scenarios side-by-side.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to the YAML model file |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `scenarios` | array of strings | yes | Scenario names to compare (e.g., `["base","optimistic","pessimistic"]`) |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_compare","arguments":{"file_path":"/path/to/model.yaml","scenarios":["base","optimistic"]}}}
```

### Engine Tools

#### `forge_simulate`

Run Monte Carlo simulation with probabilistic distributions (Normal, Triangular, Uniform, PERT, Lognormal, Discrete). Returns statistics, percentiles, threshold probabilities, and drivers (each `MC.*` input's rank correlation with each output), each output's `at_plan` (the model calculated once with every uncertain input at its plan value), and under `inputs` each input's sampled `mean` beside its `plan` and `plan_from` (the `value:` written beside its formula, or else its distribution's mode, mean, midpoint or likeliest value). An `MC.*` input may be in an included file. `outputs` is keyed by name, and a JSON object has no order; `output_order` lists the outputs in the order of `monte_carlo.outputs`, a column's rows in order.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to YAML model with `monte_carlo` config and `MC.*` distribution formulas |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `iterations` | integer | no | Number of iterations (default: from YAML config or 10000) |
| `seed` | integer | no | Random seed for reproducibility |
| `sampling` | string | no | `monte_carlo` or `latin_hypercube` (default: from config) |
| `scenario` | string | no | A scenario whose overrides apply first; an override of an `MC.*` input fixes it |
| `seek` | string | no | `"P(year.profit > 0) = 0.8"`: returns the value of `vary` at which the probability is first met, instead of the statistics |
| `vary`, `min`, `max` | string, number, number | with `seek` | The input to vary and the range to search |
| `outputs` | array of strings | no | Report only these outputs of `monte_carlo.outputs`, with the numbers they have among all of them; a name can be written without its group when only one output has it. Not with `seek` |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_simulate","arguments":{"file_path":"/path/to/monte-carlo.yaml","iterations":10000,"seed":42}}}
```

#### `forge_scenarios`

Run probability-weighted scenario analysis (Base/Bull/Bear). Each scenario overrides scalar values and calculates results. A scenario without a probability is left out of the weighting and named in `unweighted`; forge_compare puts it beside the others.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to YAML model with `scenarios` section |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `scenario_filter` | string | no | Run only this named scenario |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_scenarios","arguments":{"file_path":"/path/to/scenarios.yaml"}}}
```

#### `forge_decision_tree`

Analyze decision trees using backward induction. Returns optimal path, expected value, decision policy, and risk profile.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to YAML model with `decision_tree` section |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_decision_tree","arguments":{"file_path":"/path/to/decision-tree.yaml"}}}
```

#### `forge_real_options`

Value managerial flexibility (defer/expand/abandon) using real options pricing. Returns option values, exercise probabilities, and project value with options.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to YAML model with `real_options` section |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_real_options","arguments":{"file_path":"/path/to/real-options.yaml"}}}
```

#### `forge_tornado`

Generate tornado sensitivity diagram. Varies each input one-at-a-time to show which inputs have the greatest impact on the output.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to YAML model with `tornado` section |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `output_var` | string | no | Override the output variable to analyze |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_tornado","arguments":{"file_path":"/path/to/tornado.yaml"}}}
```

#### `forge_game`

Find the equilibria of the games in a model's `games` section, and play its tournaments (ADR-068). The same as `forge game`. Each report has `kind`: `table` for a game played once (its strictly and weakly dominated strategies, pure equilibria, mixed equilibria for two players and, in a symmetric game of two strategies of any number of players, the mixes in which all play alike with `stable`, the ties of a degenerate game, each strategy's payoff against an even mix, and with a `dynamics:` key every generation's shares in `dynamics.path`, ADR-073), or `tournament` (each player's score, rank and cooperation, the match table, and the collectively stable players, as exact expected values).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to YAML model with a `games` section |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `game` | string | no | Only this game |
| `scenario` | string | no | Apply this scenario's overrides first |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_game","arguments":{"file_path":"examples/games/axelrod.yaml","game":"axelrod"}}}
```

#### `forge_bootstrap`

Non-parametric bootstrap resampling for confidence intervals. Returns original estimate, bootstrap mean, std error, bias, and confidence intervals.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to YAML model with `bootstrap` section |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `iterations` | integer | no | Override number of bootstrap iterations |
| `seed` | integer | no | Random seed for reproducibility |
| `confidence_levels` | array of numbers | no | Override confidence levels (e.g., `[0.90, 0.95, 0.99]`) |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_bootstrap","arguments":{"file_path":"/path/to/bootstrap.yaml","seed":42}}}
```

#### `forge_bayesian`

Bayesian network inference. Query posterior probabilities with optional evidence, optionally under a scenario. A prior or table entry may be a formula over the model's scalars, worked out under the scenario. Named queries in the file (`bayesian_network.queries`) give their posteriors as scalars, `bayesian_network.<query>.<state>`, which `forge_calculate` and `forge_validate` report with the other results (ADR-074).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to YAML model with `bayesian_network` section |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `query_var` | string | no | Specific variable to query (omit for all nodes) |
| `evidence` | array of strings | no | Evidence as `variable=state` pairs (e.g., `["economy=growth"]`) |
| `scenario` | string | no | A scenario whose overrides apply before the network is worked out |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_bayesian","arguments":{"file_path":"/path/to/bayesian.yaml","evidence":["economy=growth","market=bull"]}}}
```

### Discovery Tools

#### `forge_schema`

Get JSON Schema for Forge YAML model formats.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `version` | string | no | `v1` (scalar-only) or `v5` (full). Omit to list available versions. |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_schema","arguments":{"version":"v5"}}}
```

#### `forge_functions`

List all 203 supported Excel-compatible functions with descriptions and syntax. Organized by category.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_functions","arguments":{}}}
```

#### `forge_examples`

Get runnable YAML examples for all Forge capabilities.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `name` | string | no | Example name (e.g., `monte-carlo`, `scenarios`, `decision-tree`). Omit to list all. |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_examples","arguments":{"name":"monte-carlo"}}}
```

## Common Workflows

### Validate, Calculate, Audit

Standard model development cycle:

1. `forge_validate` -- check for errors
2. `forge_calculate` -- execute all formulas
3. `forge_audit` -- trace a specific variable's dependency tree

### Inline Content Workflow (Sandboxed Clients)

When the client has no filesystem access:

1. `forge_schema` -- get the YAML model structure
2. `forge_examples` -- get a template
3. `forge_validate` with `content` -- validate the generated YAML
4. `forge_calculate` with `content` + `dry_run: true` -- compute results
5. Use `calculated_content` from the response for follow-up calls

### Monte Carlo with Confidence Intervals

Combine simulation with bootstrap for robust uncertainty quantification:

1. `forge_simulate` -- run Monte Carlo with distribution formulas
2. `forge_bootstrap` -- get confidence intervals on the estimates

### Scenario Analysis Pipeline

Compare alternative futures:

1. `forge_scenarios` -- run probability-weighted scenarios
2. `forge_compare` -- compare results side-by-side

### Discovery-First Workflow

When building a new model from scratch:

1. `forge_schema` -- understand the YAML model structure
2. `forge_examples` -- get a runnable template
3. `forge_functions` -- look up available functions
4. `forge_validate` -- verify the model
5. `forge_calculate` -- compute results

## Troubleshooting

**A relative path is relative to the folder the server may use**: `--root DIR`, or the folder the server started in. With `--any-path`, it resolves from the server's working directory, which may not be what you expect, so use absolute paths.

**"... is outside DIR, the folder this server may use (--root)".** The path, or a file the model includes, resolves to somewhere outside the folder given with `--root`, or the folder the server started in. Use a path inside it, set `--root` to the folder you want, or start with `--any-path`.

**"Provide either file_path or content, not both."** You passed both a file path and inline content. Use one or the other. If you have a file on disk, use `file_path`. If you are generating YAML in a sandbox, use `content`.

**"Either file_path or content is required."** You called a tool without specifying the model source. Provide either `file_path` (for host filesystem) or `content` (for inline YAML).

**"Missing includes for namespaces: ..."** Your YAML content has `_includes` entries but you did not provide the corresponding entries in the `includes` map. Add an entry for each namespace listed in the error. The key must match the `as` field in the `_includes` entry.

**Protocol errors.** The server expects one JSON-RPC request per line on stdin. Ensure you send a newline after each request. Responses are also one JSON object per line on stdout.

**Parse errors return JSON-RPC error code -32700.** Check that your request is valid JSON with the required `jsonrpc`, `method`, and `id` fields.

**Unknown method returns -32601.** Supported methods: `initialize`, `tools/list`, `tools/call`, `ping`, `notifications/initialized`.

**Tool returns `isError: true`.** The tool executed but encountered a domain error (e.g., file not found, invalid YAML, missing section). Check the `text` field for details.

**Monte Carlo requires `monte_carlo` config.** The YAML file must have a `monte_carlo:` section and scalar formulas using `MC.*` distributions. The `enabled` key is accepted but not read: a section with `enabled: false`, or without it, still runs.

**Scenario analysis requires `scenarios` section.** The YAML file must define named scenarios under a `scenarios:` key.
