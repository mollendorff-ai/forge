# Forge with an AI

An AI is good at writing a model and poor at arithmetic it does in its head. Forge is the other way round: the AI writes the YAML, Forge calculates it, and the same file gives the same numbers whoever opens it next. This page is for someone who works with an AI, in three ways, from the least to set up to the most:

1. [A chat that can run commands](#a-chat-that-can-run-commands): paste one prompt.
2. [The MCP server in an AI client](#the-mcp-server-in-an-ai-client): the AI calls Forge's tools itself.
3. [Rules for an agent in your model repository](#rules-for-an-agent-in-your-model-repository): what to put in `AGENTS.md` or `CLAUDE.md`.

A chat that cannot run commands, or a phone with nothing installed, can still use [Forge Studio](https://forge.mollendorff.ai/): ask the AI for the YAML and open it there.

## A chat that can run commands

If the AI has a code sandbox, it needs no setup but one prompt. The [README](../../README.md#try-it-with-your-ai) has it: it names Forge exactly, installs the release binary and its examples with one line, runs the bagel shop of [lesson 1](01-first-ten-minutes.md), and asks the AI to check one result another way and to say so if it cannot run commands. [PROMPTS.md](../PROMPTS.md) has seven more, each run by a fresh agent before it went in: build a model from a description, bring a workbook in from Excel and back, budget against actual, what if, risk and decisions, games, and the price war of [lesson 6](06-what-will-they-do.md). The page ends with what to keep in a prompt of your own.

Whatever you ask, ask it to end with the YAML, every file in full with its name. That is what you keep: it opens in Studio, goes into Excel with `forge export`, and is the model the next person calculates.

## The MCP server in an AI client

`forge mcp` is a server for the [Model Context Protocol](https://modelcontextprotocol.io/): an AI client starts it, lists its 23 tools, and calls them. Nothing is typed in a terminal; the AI validates, edits, calculates, simulates and plays games on your files, and each answer is JSON it can read. [MCP.md](../MCP.md) documents every tool. Install Forge first ([lesson 1](01-first-ten-minutes.md#install-forge)), so that `forge` is on your `PATH`.

| Client | How to add it |
| --- | --- |
| Claude Desktop | Settings, Developer, Edit Config, or edit `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS) or `%APPDATA%\Claude\claude_desktop_config.json` (Windows), and add the entry below |
| Claude Code | `claude mcp add --transport stdio forge -- forge mcp` (for you, in this folder). The `--` is needed: it ends Claude's own options. `--scope project` writes `.mcp.json` in the project folder instead, to share with a team: the entry below, under `mcpServers`, and each person approves it the first time |
| Gemini CLI | `~/.gemini/settings.json`, or `.gemini/settings.json` in a project, or `gemini mcp add`; the entry below, under `mcpServers`. Name the server without an underscore (`forge`) |
| Any other client | The command is `forge` and its argument is `mcp`; it speaks JSON-RPC over standard input and output |

The entry, where a client takes a JSON file:

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

`--root` limits every file the AI can read or write to one folder: use it. A desktop app may start the server in `/` or your home folder, and a document the AI reads can tell it to do things; without `--root` it could read any model or workbook you can and write a file anywhere you can. [Keeping files inside one folder](../MCP.md#keeping-files-inside-one-folder) says what it covers and what it does not.

These are the clients whose own documentation was read for this page, Claude Code's on 2026-11-23 and Gemini CLI's from its repository. Cursor, VS Code, Codex and others read the same command and arguments from their own settings; look up where in their documentation, as the places move.

### What the AI does with it

The server tells the client what it is for when it connects (its `instructions`): the tools by name, that `_forge_version` is the model format and not Forge's version, that a formula in a group of scalars is written `{formula: "=..."}` and in a table `"=..."`, and to start from `forge_examples name=starter`. Then a request in plain words becomes calls. Asked to "make the price 17 and tell me what it does to the profit" on lesson 2's model, an agent calls `forge_edit` to set `shop.price_per_dozen`, `forge_calculate` to write the results and `forge_validate` to see that none is stale, and reads the profit from the answer.

An AI client with no access to your files, in a browser, cannot pass a `file_path`: it passes the YAML as `content` instead, and the answer holds the new text. [Inline content mode](../MCP.md#inline-content-mode) says how.

## Rules for an agent in your model repository

An AI that works in a repository of models, a coding agent or a chat with the files attached, does better with the rules written down once than with each of them told in each conversation. Most agents read a file named `AGENTS.md` in the repository's top folder, and Claude Code reads `CLAUDE.md`; either can hold this, and a `CLAUDE.md` can import `AGENTS.md` with the line `@AGENTS.md`. Copy this into it, and change the first line to name your models:

```markdown
# Working on the financial models in this repository

A model is a YAML file that Forge (https://github.com/mollendorff-ai/forge) calculates. The AI writes YAML; Forge does the arithmetic. Never calculate a result yourself and type it in.

1. Start a new model from `forge_examples name=starter`. `_forge_version` is the model's format ("5.1.0"), not Forge's version.
2. In a group of scalars a formula is `{formula: "=price * 2"}`. In a table a column formula is a bare string, `"=price * 2"`. Do not swap them.
3. Change a model with `forge_edit` (`forge edit` in a terminal), not by writing the file again. It keeps comments and layout, changes the one thing asked, and refuses a name that is not in the model.
4. After a change, run `forge_validate`. A result that no longer matches its formula is a stale value, and it is an error.
5. Results are written by `forge_calculate`, never typed. Commit a model only when `forge_validate` passes.
6. A model with `MC.` distributions is run with `forge_simulate`. `forge_calculate` refuses it; `plan: true` calculates it at its plan.
7. Check one result another way before reporting it: a hand calculation, or `forge_audit`, which shows the formula and the inputs behind it.
8. Read and write files only in the folder the server names. A relative path is relative to it.
9. Finish with the YAML of every model you changed, in full, with its file name.
```

Each rule is something the server does, not a hope, and a test in `validation/repo` (`agent_rules`) makes the calls of this table, one at a time as an agent would, against `forge mcp` on a copy of the examples, and fails if the answer is not what the rule rests on:

| Rule | What the server answers | Tools |
| --- | --- | --- |
| 1 | `forge_examples` gives the starter model, with `_forge_version: "5.1.0"`, and `forge_calculate` calculates it as it is | `forge_examples`, `forge_calculate` |
| 2 | A bare formula in a group is refused, with the form to write instead; a bare formula in a table is calculated | `forge_calculate` |
| 3 | A misspelt name is refused ("it is not in the model") and no file changes; a right one changes one line and nothing else | `forge_edit` |
| 4 | After an edit, `scalars_valid` is false and `mismatches` names `plan.revenue`, with the 5,400 it should be | `forge_calculate`, `forge_edit`, `forge_validate` |
| 5 | After `forge_calculate`, `mismatches` is empty, nothing is unstored and the file holds `value: 5400` | `forge_edit`, `forge_calculate`, `forge_validate` |
| 6 | `forge_calculate` refuses `bagel_simulate.yaml` and says to run `forge simulate`; with `plan` it gives the inputs at plan; `forge_simulate` draws | `forge_calculate`, `forge_simulate` |
| 7 | `forge_audit` gives `plan.revenue` as `=plan.price * plan.units_per_month`, and 4800 is 40 × 120 | `forge_audit` |
| 8 | A path outside the folder is refused, naming the folder | `forge_calculate` |
| 9 | With `content`, `forge_edit` answers with the whole new file, and `forge_calculate` with `calculated_content` | `forge_edit`, `forge_calculate` |

The rules are short on purpose: the server tells a client most of them when it connects, and a long file is a file an agent reads less of. Add your own, such as which folder a model lives in, which currency it is in, and who reviews a change; keep them about your models, not about Forge.

## What this page does not do

It does not say whether a Markdown index of the documentation for AI crawlers (`llms.txt`) helps an agent: that is tested, not assumed, by roadmap item A14. And it does not cover Cursor, VS Code or Codex setup, because their documentation could not be read when it was written (A5 in the roadmap says so); the command and arguments above are the same.

Next: [the guide's index](README.md), and [WORKFLOWS.md](../WORKFLOWS.md) for ways of working with these tools in a team.
