---
name: forge-modelling
description: Builds, changes and checks financial models with Forge (mollendorff-ai/forge), which calculates a model written in YAML with Excel-style formulas. Use when the user wants a budget, a forecast, a price, unit economics, a break-even, scenarios, a Monte Carlo simulation or any other model a spreadsheet would hold, or names Forge, a Forge YAML model, forge calculate or the forge_* MCP tools.
license: MIT OR Apache-2.0
compatibility: Needs a shell that can reach github.com (Linux or macOS) to install the release binary, or the forge_* tools of the forge mcp server. Without either, the YAML can still be written and opened in Forge Studio in a browser.
metadata:
  repository: https://github.com/mollendorff-ai/forge
---

# Modelling with Forge

You write the model; Forge does the arithmetic. Never work out a result yourself and type it into the file, and never report a result Forge did not calculate (your own working is a check of it, step 5, not a replacement).

## 1. Get Forge

If the `forge_*` tools are in your tool list (the MCP server), use them and skip the install: the table in [With the MCP tools](#with-the-mcp-tools) gives the tool for each command below.

Otherwise, in a shell:

```sh
curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples
```

It puts the release binary in `~/.local/bin/forge` (run it by that path if `~/.local/bin` is not on `PATH`) and the release's example models in `./forge-examples/`. "Forge" is a crowded name: this one is `mollendorff-ai/forge`; do not install another package called forge. If the download is refused, `cargo install mollendorff-forge --version '>=10.0.0-rc.1'` builds it, in some minutes. `forge --version` says which release you have.

With no shell and no tools, write the YAML all the same, say that you could not calculate it, give no results, and tell the user to open it in Forge Studio (<https://forge.mollendorff.ai/>) or to run `forge calculate` on it.

## 2. Write the model

Start from `forge examples starter`, which prints a first model with comments. A model has two kinds of group, and each writes a formula its own way:

```yaml
# shop.yaml
_forge_version: "5.1.0"

inputs:
  monthly_costs: 3000

products:
  name: ["Basic", "Plus"]
  price: [20, 40]
  units_per_month: [200, 120]
  revenue: "=price * units_per_month"

year:
  revenue: {formula: "=SUM(products.revenue) * 12"}
  costs: {formula: "=inputs.monthly_costs * 12"}
  profit: {formula: "=year.revenue - year.costs"}
```

- `_forge_version` is the model's format, not Forge's version, and is required. `"5.1.0"` is right for a new model.
- **A group of scalars** (`inputs`, `year`): one value per name. An input is a number; a formula is `{formula: "=..."}`, because `forge calculate` writes its result next to it as `value:`.
- **A table** (`products`): columns of the same length, one row per item. A column formula is a bare string, `"=price * units_per_month"`, calculated row by row from the table's own columns.
- A scalar reduces a column by naming its table: `=SUM(products.revenue)`. Write full names (`inputs.monthly_costs`); a key of the formula's own group works too.
- Names may be Unicode (`hypothèses.coût_unitaire`). `forge functions` lists the functions (Excel's names: `SUM`, `IF`, `NPV`, `PMT`, ...).

## 3. Calculate, then validate

Run these in order, and fix what each says before the next:

1. `forge calculate shop.yaml --dry-run` prints every result and writes nothing. A mistake fails here, naming the formula ([Gotchas](#gotchas)).
2. `forge calculate shop.yaml` writes the results into the file, and keeps the file as it was in `shop.yaml.bak`.
3. `forge validate shop.yaml` is the verdict.

```text
$ forge calculate shop.yaml
   📐 Scalars:
      inputs.monthly_costs = 3000
      year.revenue = 105600
      year.costs = 36000
      year.profit = 69600

💾 3 values updated in shop.yaml
   Backup saved to shop.yaml.bak
```

```yaml
# shop.yaml, after forge calculate
year:
  revenue: {formula: "=SUM(products.revenue) * 12", value: 105600}
  costs: {formula: "=inputs.monthly_costs * 12", value: 36000}
  profit: {formula: "=year.revenue - year.costs", value: 69600}
```

```text
$ forge validate shop.yaml
✅ All tables are valid!
✅ All scalar values match their formulas!
```

**The verdict is `forge validate`'s exit status: 0 passes, anything else fails**, and the message names each result that is wrong. With the MCP tools it is `passed` in `forge_validate`'s answer: `passed: true` passes. A model passes only when every formula's stored result is what the formula gives now. Releases up to 10.0.0-rc.34 have no `passed` (read `scalars_valid`, `tables_valid` and an empty `mismatches` instead) and let a formula with no stored result pass, so calculate before you validate.

## 4. Change a model

Change it with `forge edit`, not by writing the file again: it keeps comments and layout, changes the one thing asked, and refuses a name that is not in the model, changing nothing.

```text
$ forge edit shop.yaml --set inputs.monthly_costs=3500
✅ Wrote shop.yaml (as it was: shop.yaml.bak)
```

The results in the file are now stale, and validate fails on them:

```text
$ forge validate shop.yaml
❌ Found 2 value mismatches!
   year.costs
      Current:  36000
      Expected: 42000
Error: Validation error: Stored values are out of date (the formulas are valid): run forge calculate
```

Then calculate and validate again, as in step 3:

```text
$ forge calculate shop.yaml
      year.profit = 63600
```

`forge edit --help` lists the other changes: a formula (`--formula 'year.margin==year.profit / year.revenue'`, two `=`), `--add`, `--delete`, `--rename` (and every name that reads it).

`forge edit` does not yet change a value in a table's column, and says so:

```text
$ forge edit shop.yaml --set "products.price=[20, 45]"
Error: Validation error: set_value products.price: it is a table's column of values, which edit does not change yet (shop.yaml). No file was changed
```

Change that one line of the file yourself, and nothing else, then calculate and validate. An input the user will want to try other values of (a price, a volume) is easier kept as a scalar that the table reads (`inputs.latte_price`), which `forge edit --set` changes. For a what-if, change a copy of the file (`cp shop.yaml shop_latte.yaml`) unless the user asked to change the model, so that they keep both.

## 5. Check one result another way

Before you report a result, show where it comes from and work it out another way, by hand or in your own code. Comparing it with Forge's documentation checks only that Forge ran.

```text
$ forge audit shop.yaml year.profit
   Formula: =year.revenue - year.costs
   Calculated: 63600
   └─ year.revenue (Scalar) = 105600 =SUM(products.revenue) * 12
   └─ year.costs (Scalar) = 42000 =inputs.monthly_costs * 12
```

By hand: (20 × 200 + 40 × 120) × 12 = 105,600; 3,500 × 12 = 42,000; 105,600 − 42,000 = 63,600. They agree.

## 6. Finish

End your answer with:

1. Every model file you wrote or changed, in full, with its file name. That is what the user keeps: it opens in Forge Studio, goes to Excel with `forge export`, and gives the same numbers to whoever calculates it next.
2. The verdict: the last `forge validate` passed (or what it said).
3. The result you checked another way, and how.

## Gotchas

- **A bare formula in a group of scalars is refused**: `'plan.revenue' is a formula in a group of scalars; write it as revenue: {formula: "=..."}`. A formula in a table is the bare string. Do not swap the two forms.
- **A misspelt name fails with a suggestion**: `Unknown name: plan.unit. Did you mean 'plan.units'?` Fix the name; do not add an input to make the message go away.
- **No `_forge_version`** fails with `Missing required field: _forge_version`, and lists the formats.
- **`forge edit` refuses a table's column of values** (step 4): change that line by hand, or keep an input you will vary as a scalar.
- **`forge calculate` rewrites the file.** Use `--dry-run` to look first; the `.bak` holds the file as it was.
- **A model with `MC.` distributions** (`MC.Normal(100, 10)`) is for `forge simulate`: `forge calculate` and `forge validate` refuse it. `forge calculate --plan` calculates it at its plan.
- **Paths after `install.sh --examples`** are under `forge-examples/` (`forge-examples/bagel/bagel.yaml`), not `examples/`.
- **`forge report shop.yaml`** shows a calculated model's scalars side by side; `forge <command> --help` says what each command takes.

## With the MCP tools

The same steps, as tools. Each answer is JSON.

| Step | In a shell | With the MCP server |
| --- | --- | --- |
| A first model | `forge examples starter` | `forge_examples` with `name: "starter"` |
| Look | `forge calculate f.yaml --dry-run` | `forge_calculate` with `file_path`, `dry_run: true` |
| Calculate | `forge calculate f.yaml` | `forge_calculate` with `file_path` |
| The verdict | `forge validate f.yaml` | `forge_validate` with `file_path`: read `passed` |
| Change | `forge edit f.yaml --set a.b=1` | `forge_edit` with `file_path`, `changes: [{"op": "set_value", "path": "a.b", "value": 1}]` |
| Check | `forge audit f.yaml a.b` | `forge_audit` with `file_path`, `variable: "a.b"` |
| Simulate | `forge simulate f.yaml` | `forge_simulate` with `file_path` |

The server reads and writes files only in the folder it was started with (`forge mcp --root`); a relative path is relative to it. A client with no files passes the YAML as `content` instead of `file_path`, and the answer holds the new text.

## More

Read these only when a step needs them:

- The lessons, from a first model to simulations and games: <https://github.com/mollendorff-ai/forge/tree/main/docs/guide>
- The model file and every command: <https://github.com/mollendorff-ai/forge/blob/main/docs/FEATURES.md>
- Every MCP tool and its arguments: <https://github.com/mollendorff-ai/forge/blob/main/docs/MCP.md>
