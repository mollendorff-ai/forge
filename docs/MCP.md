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
| `file_path` | string | yes | Path to the YAML model file to validate |
| `verbose` | boolean | no | Show verbose output (default: false) |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_validate","arguments":{"file_path":"/path/to/model.yaml"}}}
```

#### `forge_calculate`

Calculate all formulas in a Forge YAML model and optionally update the file.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to the YAML model file |
| `dry_run` | boolean | no | Perform a dry run without updating file (default: false) |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_calculate","arguments":{"file_path":"/path/to/model.yaml","dry_run":true}}}
```

#### `forge_audit`

Audit a specific variable to see its dependency tree and calculated value.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to the YAML model file |
| `variable` | string | yes | Name of the variable to audit |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_audit","arguments":{"file_path":"/path/to/model.yaml","variable":"assumptions.profit"}}}
```

#### `forge_export`

Export a Forge YAML model to an Excel workbook.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `yaml_path` | string | yes | Path to the YAML model file |
| `excel_path` | string | yes | Path for the output Excel file |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_export","arguments":{"yaml_path":"/path/to/model.yaml","excel_path":"/path/to/output.xlsx"}}}
```

#### `forge_import`

Import an Excel workbook into a Forge YAML model.

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
| `file_path` | string | yes | Path to the YAML model file |
| `vary` | string | yes | Name of the input variable to vary |
| `range` | string | yes | Range: start,end,step (e.g., `80,120,10`) |
| `output` | string | yes | Name of the output variable to observe |
| `vary2` | string | no | Second variable for 2D analysis |
| `range2` | string | no | Range for second variable |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_sensitivity","arguments":{"file_path":"/path/to/model.yaml","vary":"price","range":"80,120,10","output":"profit"}}}
```

#### `forge_goal_seek`

Find the input value needed to achieve a target output. Uses bisection solver.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to the YAML model file |
| `target` | string | yes | Name of the target output variable |
| `value` | number | yes | Desired value for the target |
| `vary` | string | yes | Name of the input variable to adjust |
| `min` | number | no | Minimum bound for search |
| `max` | number | no | Maximum bound for search |
| `tolerance` | number | no | Solution tolerance (default: 0.0001) |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_goal_seek","arguments":{"file_path":"/path/to/model.yaml","target":"profit","value":100000,"vary":"price"}}}
```

#### `forge_break_even`

Find the break-even point where an output equals zero.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to the YAML model file |
| `output` | string | yes | Name of the output variable to find zero crossing |
| `vary` | string | yes | Name of the input variable to adjust |
| `min` | number | no | Minimum bound for search |
| `max` | number | no | Maximum bound for search |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_break_even","arguments":{"file_path":"/path/to/model.yaml","output":"profit","vary":"units"}}}
```

#### `forge_variance`

Compare budget vs actual with variance analysis.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `budget_path` | string | yes | Path to the budget YAML file |
| `actual_path` | string | yes | Path to the actual YAML file |
| `threshold` | number | no | Variance threshold percentage for alerts (default: 10) |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_variance","arguments":{"budget_path":"/path/to/budget.yaml","actual_path":"/path/to/actual.yaml"}}}
```

#### `forge_compare`

Compare calculation results across multiple scenarios side-by-side.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to the YAML model file |
| `scenarios` | array of strings | yes | Scenario names to compare (e.g., `["base","optimistic","pessimistic"]`) |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_compare","arguments":{"file_path":"/path/to/model.yaml","scenarios":["base","optimistic"]}}}
```

### Engine Tools

#### `forge_simulate`

Run Monte Carlo simulation with probabilistic distributions (Normal, Triangular, Uniform, PERT, Lognormal). Returns statistics, percentiles, and threshold probabilities.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to YAML model with `monte_carlo` config and `MC.*` distribution formulas |
| `iterations` | integer | no | Number of iterations (default: from YAML config or 10000) |
| `seed` | integer | no | Random seed for reproducibility |
| `sampling` | string | no | `random` or `latin_hypercube` (default: from config) |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_simulate","arguments":{"file_path":"/path/to/monte-carlo.yaml","iterations":10000,"seed":42}}}
```

#### `forge_scenarios`

Run probability-weighted scenario analysis (Base/Bull/Bear). Each scenario overrides scalar values and calculates results.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to YAML model with `scenarios` section |
| `scenario_filter` | string | no | Run only this named scenario |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_scenarios","arguments":{"file_path":"/path/to/scenarios.yaml"}}}
```

#### `forge_decision_tree`

Analyze decision trees using backward induction. Returns optimal path, expected value, decision policy, and risk profile.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to YAML model with `decision_tree` section |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_decision_tree","arguments":{"file_path":"/path/to/decision-tree.yaml"}}}
```

#### `forge_real_options`

Value managerial flexibility (defer/expand/abandon) using real options pricing. Returns option values, exercise probabilities, and project value with options.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to YAML model with `real_options` section |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_real_options","arguments":{"file_path":"/path/to/real-options.yaml"}}}
```

#### `forge_tornado`

Generate tornado sensitivity diagram. Varies each input one-at-a-time to show which inputs have the greatest impact on the output.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to YAML model with `tornado` section |
| `output_var` | string | no | Override the output variable to analyze |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_tornado","arguments":{"file_path":"/path/to/tornado.yaml"}}}
```

#### `forge_bootstrap`

Non-parametric bootstrap resampling for confidence intervals. Returns original estimate, bootstrap mean, std error, bias, and confidence intervals.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to YAML model with `bootstrap` section |
| `iterations` | integer | no | Override number of bootstrap iterations |
| `seed` | integer | no | Random seed for reproducibility |
| `confidence_levels` | array of numbers | no | Override confidence levels (e.g., `[0.90, 0.95, 0.99]`) |

```json
{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"forge_bootstrap","arguments":{"file_path":"/path/to/bootstrap.yaml","seed":42}}}
```

#### `forge_bayesian`

Bayesian network inference. Query posterior probabilities with optional evidence.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | yes | Path to YAML model with `bayesian_network` section |
| `query_var` | string | no | Specific variable to query (omit for all nodes) |
| `evidence` | array of strings | no | Evidence as `variable=state` pairs (e.g., `["economy=growth"]`) |

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

**Protocol errors.** The server expects one JSON-RPC request per line on stdin. Ensure you send a newline after each request. Responses are also one JSON object per line on stdout.

**Parse errors return JSON-RPC error code -32700.** Check that your request is valid JSON with the required `jsonrpc`, `method`, and `id` fields.

**Unknown method returns -32601.** Supported methods: `initialize`, `tools/list`, `tools/call`, `ping`, `notifications/initialized`.

**Tool returns `isError: true`.** The tool executed but encountered a domain error (e.g., file not found, invalid YAML, missing section). Check the `text` field for details.

**Monte Carlo requires `monte_carlo` config.** The YAML file must have a `monte_carlo:` section with `enabled: true` and scalar formulas using `MC.*` distributions.

**Scenario analysis requires `scenarios` section.** The YAML file must define named scenarios under a `scenarios:` key.
