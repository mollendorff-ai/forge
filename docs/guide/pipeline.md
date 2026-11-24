# Models checked on every pull request

A model kept in git ([lesson 3](03-keeping-a-model.md)) is changed in pull requests: someone raises the price, recalculates, and asks for a review. This page is a workflow that checks each one before anyone reads it, and the commands it is made of. It fails a pull request whose stored results are out of date, or that adds a formula without its result, and it writes the results as JSON for the program that comes next. The workflow is [`examples/pipeline/forge.yml`](../../examples/pipeline/forge.yml); copy it to `.github/workflows/forge.yml` in your repository of models.

The commands here run on the two files of [lesson 2](02-a-first-model.md), `bagel.yaml` and `costs.yaml`, in the top folder of a repository.

<details>
<summary>The two files</summary>

```yaml
# bagel.yaml, as examples/guide/02-bagel.yaml has it
_forge_version: "5.1.0"
# Bagel Lune's first year: the model of lesson 2 of the guide
_includes:
  - file: costs.yaml
    as: costs

shop:
  price_per_dozen: 16
  dozens_per_day: 55
  monthly_growth: 0.03
  open_days_per_month: 30
  staff_hours_per_day: 32
  rent_per_month: 4800

months:
  month: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
  season: [0.85, 1.25, 0.80, 0.80, 0.90, 0.95, 1.05, 1.15, 1.25, 1.25, 1.05, 1.00]
  dozens: "=shop.dozens_per_day * (1 + shop.monthly_growth) ^ (month - 1) * season * shop.open_days_per_month"
  revenue: "=dozens * shop.price_per_dozen"
  ingredients: "=dozens * @costs.ingredients.per_dozen"
  labour: "=shop.staff_hours_per_day * shop.open_days_per_month * @costs.labour.loaded_wage"
  profit: "=revenue - ingredients - labour - shop.rent_per_month"
  cash: "=PREV(cash, 0) + profit"

year:
  revenue: {formula: "=SUM(months.revenue)"}
  profit: {formula: "=SUM(months.profit)"}
  margin: {formula: "=profit / revenue"}
  worst_month: {formula: "=MIN(months.profit)"}

scenarios:
  base:
    probability: 0.6
    description: The plan as written
    scalars: {}
  rainy_year:
    probability: 0.3
    description: A slow start and a wet summer
    scalars:
      shop.dozens_per_day: 45
  flour_shock:
    probability: 0.1
    description: The price of flour up by half
    scalars:
      "@costs.ingredients.flour_per_dozen": 2.30
```

```yaml
# costs.yaml, as examples/guide/costs.yaml has it
_forge_version: "5.1.0"
# What the bagel shop pays its suppliers and its staff. When a price
# changes, this is the file that changes.
ingredients:
  flour_per_dozen: 1.53
  sesame_per_dozen: 0.68
  honey_water_per_dozen: 0.35
  per_dozen: {formula: "=flour_per_dozen + sesame_per_dozen + honey_water_per_dozen"}
labour:
  wage_per_hour: 18.50
  payroll_load: 0.15
  loaded_wage: {formula: "=wage_per_hour * (1 + payroll_load)"}
```

</details>

## The gate: forge validate

A repository of models keeps each result in the file next to its formula, so that a reviewer reads the numbers in the pull request without running Forge. Calculate both files, as the person who opens the pull request does:

```text
$ forge calculate bagel.yaml
💾 4 values updated in bagel.yaml
```

```text
$ forge calculate costs.yaml
💾 2 values updated in costs.yaml
```

`forge validate` calculates every formula again and compares it with the result stored next to it, and checks what each file states it should give (an [`expect:`](../FEATURES.md#stating-what-a-scenario-gives-expect-format-560-adr-074), in format 5.6.0). Given every model of the repository, it reports on each:

```text
$ forge validate bagel.yaml costs.yaml
   ✅ bagel.yaml - OK
   ✅ costs.yaml - OK
   2 passed, 0 failed out of 2 files
```

What a job reads is the exit code, which is the same for every command:

| Exit code | What it means |
| --- | --- |
| 0 | It did what it was asked; for `forge validate`, every file passed |
| 1 | It failed, and the last line it wrote to standard error starts with `Error:` and says why: a stored result out of date, a formula that fails, an `expect:` not met, a file that is not there |
| 2 | The command line is wrong (an option misspelt, a file not given); it says so and prints its usage |

Now someone changes the price and opens a pull request without calculating:

```text
$ forge edit bagel.yaml --set shop.price_per_dozen=17
✅ Wrote bagel.yaml (as it was: bagel.yaml.bak)
```

```text
$ forge validate bagel.yaml costs.yaml
   year.profit
      Current:  22247.057612
      Expected: 46423.773208
   ❌ bagel.yaml - FAILED
   ✅ costs.yaml - OK
   1 passed, 1 failed out of 2 files
Error: Validation error: 1 file(s) failed validation
```

It exits with 1, and the job stops there, before anything reads the numbers that are wrong. `forge calculate bagel.yaml` writes the new ones, and the pull request passes.

```text
$ forge calculate bagel.yaml
💾 4 values updated in bagel.yaml
```

## A formula with no result: the JSON

`forge validate` compares the results that are there. A formula with no result yet has nothing to compare, and passes, with a line that says so; that is how a model is written, a formula at a time, before it is first calculated. In a repository, though, a pull request that adds a formula and does not calculate it leaves a reviewer a formula with no number:

```text
$ forge edit bagel.yaml --add "year.food_cost_share==SUM(months.ingredients) / revenue"
✅ Wrote bagel.yaml (as it was: bagel.yaml.bak)
```

```text
$ forge validate bagel.yaml costs.yaml
   1 formula(s) have no stored value yet; run 'forge calculate' to store them
   ✅ bagel.yaml - OK
   2 passed, 0 failed out of 2 files
```

`forge report` prints the results stored in a file, and never calculates. With `--format json` it is for a program: each group of results with its values, a value not yet calculated as `null`, and the formulas with no result under `uncalculated`:

```text
$ forge report bagel.yaml --format json
          "group": "year",
            "food_cost_share": null,
            "profit": 46423.7732078896,
  "uncalculated": [
    "year.food_cost_share"
  ]
```

So the workflow reads `uncalculated` with `jq` and fails when it is not empty. Calculated, it is:

```text
$ forge calculate bagel.yaml
💾 1 value updated in bagel.yaml
```

```text
$ forge report bagel.yaml --format json
            "food_cost_share": 0.150588235294118,
  "uncalculated": []
```

`--format csv` gives the same numbers for a spreadsheet or a database, and `--format markdown` the tables a person reads.

## The workflow

```yaml
# examples/pipeline/forge.yml
# A repository of Forge models, checked on every pull request: copy this
# file to .github/workflows/forge.yml. It fails when a stored result is out
# of date, when a formula has no stored result, or when an expect: is not
# met; it writes each model's results as JSON for the next program, and
# shows them on the run's page. docs/guide/pipeline.md explains each step.
name: Models

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read

defaults:
  run:
    shell: bash

env:
  # The release to install; raise it in a pull request of its own
  FORGE_VERSION: v10.0.0-rc.34

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7

      - name: Install Forge
        run: |
          curl -fsSL "https://raw.githubusercontent.com/mollendorff-ai/forge/$FORGE_VERSION/install.sh" | sh
          echo "$HOME/.local/bin" >> "$GITHUB_PATH"

      - name: Every stored result is what its formula gives
        run: forge validate *.yaml

      - name: Every formula has its result stored
        run: |
          mkdir -p results
          for model in *.yaml; do
            forge report "$model" --format json > "results/${model%.yaml}.json"
            if ! jq -e '.uncalculated == []' "results/${model%.yaml}.json" > /dev/null; then
              echo "::error file=$model::no stored result for $(jq -r '.uncalculated | join(", ")' "results/${model%.yaml}.json"): run forge calculate $model"
              exit 1
            fi
          done

      - name: The results, on the run's page
        run: |
          for model in *.yaml; do
            echo "### $model"
            echo
            forge report "$model" --format markdown
            echo
          done >> "$GITHUB_STEP_SUMMARY"

      - uses: actions/upload-artifact@cf430e030ddbb5b0abf93d22962f4752f3646cd9 # v7
        with:
          name: results
          path: results/
```

It runs on each pull request and on each push to `main`, with read-only access to the repository:

1. **Install Forge**: `install.sh` downloads the release `FORGE_VERSION` names, checks it against the release's checksums, and puts `forge` in `~/.local/bin`. The version is pinned, so a new release of Forge changes nothing until a pull request raises it, and that pull request shows whether every model still passes.
2. **Every stored result is what its formula gives**: `forge validate` on every `.yaml` file at the top of the repository. A result out of date, a formula that fails or an `expect:` not met exits with 1, and the run stops.
3. **Every formula has its result stored**: `forge report --format json` for each model, into `results/`, and the run stops at the first file whose `uncalculated` is not empty, naming the file and the formulas on the pull request (the `::error` line is how a step annotates a file).
4. **The results, on the run's page**: `forge report --format markdown` for each model, in the run's summary, where a reviewer reads them without opening a file.
5. **The JSON** is kept with the run as an artifact named `results`, for the next job or program.

Models in a folder of their own, or with names other than `*.yaml`, need the three `*.yaml` changed to that folder; a model that is only included by another is checked with it, and on its own too, since every file is given.

## What a pull request shows

A change to a model is a change to its text, and the results it moved are in the same commit, so the pull request's diff is the review: the price's line and the four results it moved, in `bagel.yaml`, as the commands above left them.

```yaml
# bagel.yaml, after forge calculate
  price_per_dozen: 17
```

```yaml
# bagel.yaml, after forge calculate
  revenue: {formula: "=SUM(months.revenue)", value: 411004.165133942}
  profit: {formula: "=SUM(months.profit)", value: 46423.7732078896}
  margin: {formula: "=profit / revenue", value: 0.112952074811117}
  worst_month: {formula: "=MIN(months.profit)", value: -5002.39728}
```

## It has run

This workflow has run, unchanged, in [a repository of its own](https://github.com/mollendorff-ai/forge-pipeline-example) holding these two files, calculated with the release it installs (2026-11-24):

| Pull request | What it changes | The run |
| --- | --- | --- |
| [#1](https://github.com/mollendorff-ai/forge-pipeline-example/pull/1) | The price to 17, calculated | [Passed](https://github.com/mollendorff-ai/forge-pipeline-example/actions/runs/45303977313), with the results on its page and as JSON |
| [#2](https://github.com/mollendorff-ai/forge-pipeline-example/pull/2) | The price to 17, not calculated | [Failed](https://github.com/mollendorff-ai/forge-pipeline-example/actions/runs/45303984249) at "Every stored result is what its formula gives" |
| [#3](https://github.com/mollendorff-ai/forge-pipeline-example/pull/3) | `year.food_cost_share` added, not calculated | [Failed](https://github.com/mollendorff-ai/forge-pipeline-example/actions/runs/45303990085) at "Every formula has its result stored", after `forge validate` passed |

[The push](https://github.com/mollendorff-ai/forge-pipeline-example/actions/runs/45303934481) of the two files to `main` passed as well. #1's [diff](https://github.com/mollendorff-ai/forge-pipeline-example/pull/1/files) is the one above. The tests of this repository run the same steps on the same files with the `forge` of its sources, green and red, on every pull request (`validation/repo`'s `pipeline` module), so a change to Forge that would break the workflow fails here first.

## Check it yourself

The JSON says the year's profit at 17 a dozen is 46,423.77. [Lesson 3](03-keeping-a-model.md#check-it-yourself) worked out that a dollar more a dozen adds 24,176.715596 to the year's revenue, and so to its profit, which was 22,247.057612: 22,247.057612 + 24,176.715596 = 46,423.773208, what `forge validate` expected above.

## Your turn

A pull request raises the rent to 5,200 and calculates. Which steps of the workflow pass, and what is the year's profit in the JSON?

<details>
<summary>The answer</summary>

```text
$ forge edit bagel.yaml --set shop.rent_per_month=5200
✅ Wrote bagel.yaml (as it was: bagel.yaml.bak)
```

```text
$ forge calculate bagel.yaml
💾 3 values updated in bagel.yaml
```

```text
$ forge report bagel.yaml --format json
            "profit": 41623.7732078896,
  "uncalculated": []
```

Every step passes: the results are calculated, so `forge validate` finds none out of date and `uncalculated` is empty. The profit is `41623.7732078896`: 400 more rent for 12 months is 4,800 less than 46,423.77.

</details>

Next: [the guide's index](README.md), and [WORKFLOWS.md](../WORKFLOWS.md#3-a-pipeline-without-ai) for the rest of a pipeline: `forge variance` against a budget, and `forge export` for the finance team.
