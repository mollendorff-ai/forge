# Workflows: four ways to use Forge

Forge calculates a financial model written as YAML. The same model, and the same binary, serve four ways of working, from one person in an editor to an AI pipeline that nobody watches. This page shows each one with a diagram and the commands it uses.

**What is proven and what is not.** The commands on this page were run against Forge 10.0.0-rc.26 on 2026-11-15, and the chat workflow was run end to end by fresh agents ([PROMPTS.md](PROMPTS.md)). The two pipeline workflows are architecture: every step in them is a Forge command that works, but no one has yet run a whole pipeline in production. The AI pipeline is to be demonstrated by Sentinel, a small earnings-analysis pipeline built on Forge (roadmap item T2-PIPELINE-DEMO).

## The big picture

```mermaid
flowchart LR
    person([A person])
    ai([An AI in a chat])
    job([A scheduled job])
    agent([An AI agent in a pipeline])

    subgraph forge [Forge, one binary]
        cli[Command line]
        mcp[MCP server, 20 tools]
        rest[REST API]
        studio[Studio in the browser, WebAssembly]
    end

    yaml[(YAML models in git)]
    xlsx[Excel workbooks, live formulas]
    out[Reports: table, Markdown, CSV, JSON]

    person --> studio
    person --> cli
    ai --> cli
    ai --> mcp
    job --> cli
    job --> rest
    agent --> cli
    agent --> mcp

    cli <--> yaml
    mcp <--> yaml
    rest <--> yaml
    studio <--> yaml
    cli --> xlsx
    cli --> out
```

Whoever writes the model, Forge does the arithmetic, the model is plain text in git, and every number traces back to a formula. The four entry points share one engine: Studio runs the same code compiled to WebAssembly, and CI checks that it gives the same answers as the binary.

## 1. A person alone

No AI needed. A model is a text file, so it can be reviewed, diffed and kept in git like code, and a change shows as a change to a formula or an input, not as a cell that looks the same.

```mermaid
flowchart LR
    edit[Edit the YAML<br/>in an editor or in Studio] --> calc[forge calculate]
    calc --> diff[git diff:<br/>which inputs and results changed]
    diff --> rep[forge report:<br/>the results as a table]
    rep --> commit[git commit]
    commit --> share[forge export<br/>a workbook to share]
```

```sh
# Change an input in budget.yaml, say groceries: 700 to groceries: 750, then
forge calculate budget.yaml          # results written next to their formulas
git diff budget.yaml                 # the input and every result it moved
forge report budget.yaml             # the results as a table
git commit -am "Groceries up to 750"
forge export budget.yaml budget.xlsx # for someone who lives in Excel
```

Two things to know:
- **The first `forge calculate`** of a file that has never been calculated, like the examples, adds a `value:` under every formula, so its first diff is all additions. From then on a diff shows only what moved.
- **`forge calculate` keeps the file as it was** in `budget.yaml.bak`. Add `*.bak` to `.gitignore`, so it is never committed.

On a phone, [Forge Studio](https://forge.mollendorff.ai/) does the same with nothing installed: paste or open the YAML, change an input, see the results move.

## 2. A person with an AI in a chat

The AI turns a request in words into a model, runs Forge, and explains the result; the person asks the next question. The AI never does the arithmetic, so it cannot make a number up.

```mermaid
sequenceDiagram
    actor P as Person
    participant A as AI
    participant F as Forge
    P->>A: "A 12-month plan for my coffee shop"
    A->>F: forge validate, forge calculate
    F-->>A: results, or the formula that failed and why
    A-->>P: revenue, profit, the formula behind each number
    P->>A: "What if rent goes up in July?"
    A->>F: changed model, forge calculate
    F-->>A: new results
    A-->>P: the difference, and the YAML to keep
```

Five tested prompts to start from are in [PROMPTS.md](PROMPTS.md). The AI can call Forge from its shell or through the MCP server (`forge mcp`, see [MCP.md](MCP.md)).

## 3. A pipeline without AI

Forge as a deterministic calculator in a financial pipeline: the model is fixed and reviewed, and a job feeds it new inputs, say the month's actuals at month-end. The same inputs always give the same numbers, and with `set -e` the run stops on the first command that fails.

```mermaid
flowchart TD
    data[New inputs arrive<br/>bank export, ledger, a form] --> write[A script writes them<br/>into actual.yaml]
    write --> calc{forge calculate actual.yaml}
    calc -- exit 1: a formula failed --> stop[Stop the run,<br/>tell a person]
    calc -- exit 0 --> var[forge variance budget.yaml actual.yaml<br/>--threshold 10 -o variance.yaml]
    var --> rep[forge report actual.yaml --format json<br/>> actual.json]
    rep --> xl[forge export actual.yaml actual.xlsx]
    xl --> git[git commit: the inputs, the results<br/>and the variance, together]
```

```sh
set -e                                                    # stop on the first failure
# The job has written this month's figures into actual.yaml
forge calculate actual.yaml                               # exit 1 on any failed formula
forge variance budget.yaml actual.yaml --threshold 10 -o variance.yaml
forge report actual.yaml --format json > actual.json      # for the next program
forge export actual.yaml actual.xlsx                      # for the finance team
git add actual.yaml variance.yaml actual.json actual.xlsx
git commit -m "Month-end actuals"
```

What makes Forge fit here:
- **Exit codes:** `forge calculate` and `forge validate` exit with 1 when a formula fails, and name the formula and what depends on it. A failed `forge calculate` leaves the file as it was.
- **A gate for stale files:** `forge validate` exits with 1 when the values stored in a file no longer match its formulas, as after an input was changed without recalculating. Run in CI, it refuses a model whose stored results are out of date. It does not yet refuse a file with no stored values at all, which passes (roadmap T1-AGENT-FRICTION).
- **Output for programs:** `forge report --format json` or `--format csv`, and `forge variance -o variance.yaml` with each line's budget, actual, variance and whether it is good news.
- **Know what each command reads:** `forge report` shows the values stored in the file and never recalculates, so run `forge calculate` first, as above; `forge variance` calculates both files itself. `forge variance` exits with 0 however many lines are over the threshold: a job that should stop on a large variance reads `exceeds_threshold` in `variance.yaml`.
- **No runtime:** one binary, installed in seconds with `install.sh`, nothing else on the machine.
- **A server, if a script would rather not run commands:** `forge serve` gives the same operations as a REST API on this machine.

## 4. A pipeline with AI

The piece the other three lead to. An agent reads something new, an earnings report, a contract, a brief, and writes or updates a model. Forge checks and calculates it, and the agent reads Forge's answer, never its own arithmetic. When Forge refuses the model, the agent reads the reason and fixes it. A person reviews the change as a pull request: a diff of formulas and inputs, not a workbook to trust.

```mermaid
sequenceDiagram
    participant T as Trigger<br/>(schedule, new document)
    participant A as AI agent
    participant F as Forge
    participant G as git
    actor R as Reviewer
    T->>A: a new report to model
    A->>A: write or update model.yaml
    loop until Forge accepts it
        A->>F: forge validate, forge calculate
        F-->>A: exit 1 and the reason, or exit 0 and results
    end
    A->>F: forge report --format json
    F-->>A: the numbers
    A->>G: branch, commit model.yaml, open a pull request
    R->>G: review the diff of formulas and inputs, merge
    G->>F: CI: forge validate on the merged model
```

Why this is the point of Forge:
- **The AI is good at reading and writing structure, and bad at arithmetic.** YAML is a format LLMs know well, so the agent writes the model, and Forge, which is checked against Excel, Gnumeric, LibreOffice and R, does the sums.
- **Every run leaves a record:** the model as text, in git, with its results stored next to its formulas, so a number in a report can be traced to the formula and the input behind it.
- **The same answers each time:** rerun a thousand scenarios and the numbers are Forge's every time, never a model's guess.

What is not yet shown: a whole run like this, unattended, on real documents. Before that, the fresh agents that ran [PROMPTS.md](PROMPTS.md) found messages and options that a person works around and a pipeline would not; they are listed in the roadmap as T1-AGENT-FRICTION.

## Choosing an entry point

| You are | Start with |
|---|---|
| Trying Forge on a phone | [Forge Studio](https://forge.mollendorff.ai/), nothing to install |
| Building a model by hand | the command line and `forge examples starter` |
| Working with an AI in a chat | a prompt from [PROMPTS.md](PROMPTS.md) |
| Connecting an AI host (Claude Desktop, Claude Code) | `forge mcp`, see [MCP.md](MCP.md) |
| Writing a scheduled job | the command line and its exit codes, or `forge serve` |
| Building an AI pipeline | `forge mcp` or the command line, `forge validate` as the gate |
