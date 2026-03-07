# Forge MCP Integration Guide

Forge implements the [Model Context Protocol](https://modelcontextprotocol.io/) (MCP) over stdin/stdout using JSON-RPC 2.0. This gives AI agents structured access to all 20 Forge tools.

Forge uses [rmcp](https://github.com/anthropics/rust-mcp-sdk) (Anthropic's official Rust MCP SDK) for protocol handling. Tool definitions use typed request structs with automatic JSON schema generation. Additional transports (SSE, WebSocket) are planned for a future release.

## Quick Start

### Claude Desktop

Add to `~/.config/claude/claude_desktop_config.json`:

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

## Inline Content Mode

MCP clients running in sandboxed environments (Claude.ai, Cursor, containers) cannot pass a `file_path` because the path does not exist on the Forge host. These clients can pass YAML content directly using the `content` parameter instead.

All 16 file-path tools accept `content` as an alternative to `file_path`. The two are mutually exclusive -- provide one or the other, never both.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_validate","arguments":{"content":"name: Demo\ntables:\n  - name: assumptions\n    columns:\n      - name: price\n        value: 100\n      - name: cost\n        value: 60\n      - name: profit\n        formula: price - cost\n"}}}
```

The server writes the content to a temporary directory, passes the temp path to the calculation engine, and cleans up automatically when the request completes.

### Inline Includes

Forge models can reference external YAML files via `_includes`. In inline mode, pass include content through the `includes` parameter -- a map of namespace names to YAML content strings. The namespace key must match the `as` field in the model's `_includes` entries.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_calculate","arguments":{"content":"name: Main Model\n_includes:\n  - file: rates.yaml\n    as: rates\ntables:\n  - name: revenue\n    columns:\n      - name: fx_rate\n        formula: rates.assumptions.usd_eur\n      - name: sales\n        value: 1000\n      - name: revenue_eur\n        formula: sales * fx_rate\n","includes":{"rates":"name: Rates\ntables:\n  - name: assumptions\n    columns:\n      - name: usd_eur\n        value: 0.92\n"},"dry_run":true}}}
```

If the model YAML contains `_includes` entries but no `includes` map is provided, the server returns a clear error listing the missing namespaces.

### Write-Back with Inline Content

When calling `forge_calculate` with `content` and `dry_run: false`, the server cannot update a persistent file (there is none). Instead, the response includes a `calculated_content` field containing the full YAML with computed values filled in. The client can use this content for subsequent calls.

```json
{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"forge_calculate","arguments":{"content":"name: Demo\ntables:\n  - name: assumptions\n    columns:\n      - name: price\n        value: 100\n      - name: cost\n        value: 60\n      - name: profit\n        formula: price - cost\n","dry_run":false}}}
```

The response will contain both the calculation results and a `calculated_content` field with the updated YAML.

## Available Tools (20)

| Category | Count | Tools |
|----------|-------|-------|
| **Core** | 5 | validate, calculate, audit, export, import |
| **Analysis** | 5 | sensitivity, goal_seek, break_even, variance, compare |
| **Engines** | 7 | simulate, scenarios, decision_tree, real_options, tornado, bootstrap, bayesian |
| **Discovery** | 3 | schema, functions, examples |

## Tool Reference

### Core Tools

#### `forge_validate`

Validate a Forge YAML model file for formula errors, circular dependencies, and type mismatches.

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

*One of `file_path` or `content` is required. When using `content` with `dry_run: false`, the response includes a `calculated_content` field.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_calculate","arguments":{"file_path":"/path/to/model.yaml","dry_run":true}}}
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

Run Monte Carlo simulation with probabilistic distributions (Normal, Triangular, Uniform, PERT, Lognormal). Returns statistics, percentiles, and threshold probabilities.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to YAML model with `monte_carlo` config and `MC.*` distribution formulas |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `iterations` | integer | no | Number of iterations (default: from YAML config or 10000) |
| `seed` | integer | no | Random seed for reproducibility |
| `sampling` | string | no | `random` or `latin_hypercube` (default: from config) |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_simulate","arguments":{"file_path":"/path/to/monte-carlo.yaml","iterations":10000,"seed":42}}}
```

#### `forge_scenarios`

Run probability-weighted scenario analysis (Base/Bull/Bear). Each scenario overrides scalar values and calculates results.

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

Bayesian network inference. Query posterior probabilities with optional evidence.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | no* | Path to YAML model with `bayesian_network` section |
| `content` | string | no* | Raw YAML model content (alternative to `file_path`) |
| `includes` | object | no | Inline include content as namespace-to-YAML map |
| `query_var` | string | no | Specific variable to query (omit for all nodes) |
| `evidence` | array of strings | no | Evidence as `variable=state` pairs (e.g., `["economy=growth"]`) |

*One of `file_path` or `content` is required.

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_bayesian","arguments":{"file_path":"/path/to/bayesian.yaml","evidence":["economy=growth","market=bull"]}}}
```

### Discovery Tools

#### `forge_schema`

Get JSON Schema for Forge YAML model formats.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `version` | string | no | `v1` (scalar-only) or `v5` (full enterprise). Omit to list available versions. |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_schema","arguments":{"version":"v5"}}}
```

#### `forge_functions`

List all 173 supported Excel-compatible functions with descriptions and syntax. Organized by category.

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

**File paths must be absolute.** Relative paths resolve from the MCP server's working directory, which may not be what you expect. Always use absolute paths.

**"Provide either file_path or content, not both."** You passed both a file path and inline content. Use one or the other. If you have a file on disk, use `file_path`. If you are generating YAML in a sandbox, use `content`.

**"Either file_path or content is required."** You called a tool without specifying the model source. Provide either `file_path` (for host filesystem) or `content` (for inline YAML).

**"Missing includes for namespaces: ..."** Your YAML content has `_includes` entries but you did not provide the corresponding entries in the `includes` map. Add an entry for each namespace listed in the error. The key must match the `as` field in the `_includes` entry.

**Protocol errors.** The server expects one JSON-RPC request per line on stdin. Ensure you send a newline after each request. Responses are also one JSON object per line on stdout.

**Parse errors return JSON-RPC error code -32700.** Check that your request is valid JSON with the required `jsonrpc`, `method`, and `id` fields.

**Unknown method returns -32601.** Supported methods: `initialize`, `tools/list`, `tools/call`, `ping`, `notifications/initialized`.

**Tool returns `isError: true`.** The tool executed but encountered a domain error (e.g., file not found, invalid YAML, missing section). Check the `text` field for details.

**Monte Carlo requires `monte_carlo` config.** The YAML file must have a `monte_carlo:` section with `enabled: true` and scalar formulas using `MC.*` distributions.

**Scenario analysis requires `scenarios` section.** The YAML file must define named scenarios under a `scenarios:` key.
