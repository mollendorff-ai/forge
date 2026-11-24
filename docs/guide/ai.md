# Forge with an AI

An AI is good at writing a model and poor at arithmetic it does in its head. Forge is the other way round: the AI writes the YAML, Forge calculates it, and the same file gives the same numbers whoever opens it next. This page is for someone who works with an AI, in three ways, from the least to set up to the most:

1. [A chat that can run commands](#a-chat-that-can-run-commands): paste one prompt.
2. [The Forge skill](#the-forge-skill): add it once, and an agent that loads skills knows how to model with Forge whenever a task calls for it.
3. [The MCP server in an AI client](#the-mcp-server-in-an-ai-client): the AI calls Forge's tools itself.
4. [Rules for an agent in your model repository](#rules-for-an-agent-in-your-model-repository): what to put in `AGENTS.md` or `CLAUDE.md`.

A chat that cannot run commands, or a phone with nothing installed, can still use [Forge Studio](https://forge.mollendorff.ai/): ask the AI for the YAML and open it there.

## A chat that can run commands

If the AI has a code sandbox, it needs no setup but one prompt. The [README](../../README.md#try-it-with-your-ai) has it: it names Forge exactly, installs the release binary and its examples with one line, runs the bagel shop of [lesson 1](01-first-ten-minutes.md), and asks the AI to check one result another way and to say so if it cannot run commands. [PROMPTS.md](../PROMPTS.md) has seven more, each run by a fresh agent before it went in: build a model from a description, bring a workbook in from Excel and back, budget against actual, what if, risk and decisions, games, and the price war of [lesson 6](06-what-will-they-do.md). The page ends with what to keep in a prompt of your own.

Whatever you ask, ask it to end with the YAML, every file in full with its name. That is what you keep: it opens in Studio, goes into Excel with `forge export`, and is the model the next person calculates.

## The Forge skill

[`skills/forge-modelling/SKILL.md`](../../skills/forge-modelling/SKILL.md) is an [Agent Skill](https://agentskills.io/specification): a folder with one Markdown file, whose name and description an agent reads when it starts and whose steps it reads when a task needs them. It carries what a prompt of [PROMPTS.md](../PROMPTS.md#writing-your-own) carries, so you need not paste it each time: the install line, the two ways a formula is written (`{formula: "=..."}` in a group of scalars, a bare `"=..."` in a table), calculate before validate, `forge validate` as the verdict (its exit status, or `passed` from `forge_validate`), `forge edit` for a change, one result checked by hand, and the YAML at the end. It works in a shell or with the MCP server's tools, and says what to do with neither.

| Client | Where the folder goes |
| --- | --- |
| Claude Code | `.claude/skills/forge-modelling/` in a project, to share it with the repository, or `~/.claude/skills/forge-modelling/` for every project on your computer |
| Claude (claude.ai and the desktop app) | Zip the folder, then Customize, Skills, "+", Create skill, Upload a skill. On an individual plan, code execution must be on first (Settings, Capabilities) |
| Other clients that read skills | `.agents/skills/forge-modelling/` in a project or `~/.agents/skills/forge-modelling/`, where many of them look, or the folder the client's own page names |

Claude Code's row is from [its skills page](https://code.claude.com/docs/en/skills) and Claude's from [Using skills in Claude](https://support.claude.com/en/articles/12512180-using-skills-in-claude), both read on 2026-11-24; the `.agents/skills/` folders are the convention the Agent Skills site's [guide for clients](https://agentskills.io/client-implementation/adding-skills-support) describes, read from its source the same day. A test in `validation/repo` (`skill`) checks the file against the specification's rules, runs each `forge` command it shows and compares what is printed, and makes each call of its table of MCP tools against `forge mcp`.

## The MCP server in an AI client

`forge mcp` is a server for the [Model Context Protocol](https://modelcontextprotocol.io/): an AI client starts it, lists its 23 tools, and calls them. Nothing is typed in a terminal; the AI validates, edits, calculates, simulates and plays games on your files, and each answer is JSON it can read. [MCP.md](../MCP.md) documents every tool. Install Forge first ([lesson 1](01-first-ten-minutes.md#install-forge)), so that `forge` is on your `PATH`.

| Client | How to add it |
| --- | --- |
| Claude Desktop | The Claude menu in the menu bar, Settings, Developer, Edit Config, which opens `~/Library/Application Support/Claude/claude_desktop_config.json` (macOS), `%APPDATA%\Claude\claude_desktop_config.json` (Windows) or `~/.config/Claude/claude_desktop_config.json` (Linux); add the entry below, then quit Claude Desktop completely and start it again |
| Claude Code | `claude mcp add --transport stdio forge -- forge mcp` (for you, in this folder). The `--` is needed: it ends Claude's own options. `--scope project` writes `.mcp.json` in the project folder instead, to share with a team: the entry below, under `mcpServers`, and each person approves it the first time |
| Gemini CLI | `~/.gemini/settings.json`, or `.gemini/settings.json` in a project, or `gemini mcp add`; the entry below, under `mcpServers`. Name the server without an underscore (`forge`) |
| Cursor | `~/.cursor/mcp.json` for every project, or `.cursor/mcp.json` in a project; the entry below, under `mcpServers`, with `"type": "stdio"` added, which Cursor's field table marks as required |
| VS Code (GitHub Copilot) | `~/.copilot/mcp-config.json` (or `$COPILOT_HOME/mcp-config.json`) for you, or `.mcp.json` at the workspace root to share; the entry below, under `mcpServers`, with `"type": "stdio"` added. Or Chat: Open Customizations, MCP Servers, Add Server, Command (stdio). `.vscode/mcp.json`, whose key is `servers`, is the older format, for the Local harness that VS Code is retiring |
| Codex | `codex mcp add forge -- forge mcp --root /Users/you/models`, or in `~/.codex/config.toml` (or `.codex/config.toml` in a trusted project) a table `[mcp_servers.forge]` with `command = "forge"` and `args = ["mcp", "--root", "/Users/you/models"]` |
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

Each row comes from the client's own documentation, read on 2026-11-24 unless the date is given: Claude Desktop's from [Connect to local MCP servers](https://modelcontextprotocol.io/docs/develop/connect-local-servers), Claude Code's on 2026-11-23, Gemini CLI's from its repository, Cursor's from [Model Context Protocol](https://cursor.com/docs/context/mcp), VS Code's from [Add and use MCP servers](https://code.visualstudio.com/docs/copilot/customization/mcp-servers) (updated 2026-11-18) and its [MCP configuration reference](https://code.visualstudio.com/docs/copilot/reference/mcp-configuration), and Codex's from [Model Context Protocol](https://learn.chatgpt.com/docs/extend/mcp?surface=cli). These places move; if a file is not where a row says, the client's page says where it went.

### What the AI does with it

The server tells the client what it is for when it connects (its `instructions`): the tools by name, that `_forge_version` is the model format and not Forge's version, that a formula in a group of scalars is written `{formula: "=..."}` and in a table `"=..."`, and to start from `forge_examples name=starter`. Then a request in plain words becomes calls. Asked to "make the price 17 and tell me what it does to the profit" on lesson 2's model, an agent calls `forge_edit` to set `shop.price_per_dozen`, `forge_calculate` to write the results and `forge_validate` to see that none is stale, and reads the profit from the answer.

An AI client with no access to your files, in a browser, cannot pass a `file_path`: it passes the YAML as `content` instead, and the answer holds the new text. [Inline content mode](../MCP.md#inline-content-mode) says how.

## Rules for an agent in your model repository

An AI that works in a repository of models, a coding agent or a chat with the files attached, does better with the rules written down once than with each of them told in each conversation. Most agents read a file named `AGENTS.md` in the repository's top folder, and Claude Code reads `CLAUDE.md`; either can hold this, and a `CLAUDE.md` can import `AGENTS.md` with the line `@AGENTS.md`. `AGENTS.md` is plain Markdown with no required fields, and a subfolder can have one of its own: the one nearest the file being edited wins, and what you say in the chat wins over both ([agents.md](https://agents.md/), read on 2026-11-24). Copy this into it, and change the first line to name your models:

```markdown
# Working on the financial models in this repository

A model is a YAML file that Forge (https://github.com/mollendorff-ai/forge) calculates. The AI writes YAML; Forge does the arithmetic. Never calculate a result yourself and type it in.

1. Start a new model from `forge_examples name=starter`. `_forge_version` is the model's format ("5.1.0"), not Forge's version.
2. In a group of scalars a formula is `{formula: "=price * 2"}`. In a table a column formula is a bare string, `"=price * 2"`. Do not swap them.
3. Change a model with `forge_edit` (`forge edit` in a terminal), not by writing the file again. It keeps comments and layout, changes the one thing asked, and refuses a name that is not in the model.
4. After a change, run `forge_validate` and read `passed`. A result that no longer matches its formula is a stale value, a formula with no result is not calculated yet (`uncalculated` names it), and either fails.
5. Results are written by `forge_calculate`, never typed. Commit a model only when `forge_validate` gives `passed: true`.
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
| 4 | After an edit, `passed` and `scalars_valid` are false and `mismatches` names `plan.revenue`, with the 5,400 it should be | `forge_calculate`, `forge_edit`, `forge_validate` |
| 5 | After `forge_calculate`, `passed` is true, `mismatches` and `uncalculated` are empty, and the file holds `value: 5400` | `forge_edit`, `forge_calculate`, `forge_validate` |
| 6 | `forge_calculate` refuses `bagel_simulate.yaml` and says to run `forge simulate`; with `plan` it gives the inputs at plan; `forge_simulate` draws | `forge_calculate`, `forge_simulate` |
| 7 | `forge_audit` gives `plan.revenue` as `=plan.price * plan.units_per_month`, and 4800 is 40 × 120 | `forge_audit` |
| 8 | A path outside the folder is refused, naming the folder | `forge_calculate` |
| 9 | With `content`, `forge_edit` answers with the whole new file, and `forge_calculate` with `calculated_content` | `forge_edit`, `forge_calculate` |

The rules are short on purpose: the server tells a client most of them when it connects, and a long file is a file an agent reads less of. Add your own, such as which folder a model lives in, which currency it is in, and who reviews a change; keep them about your models, not about Forge.

## What this page does not do

It does not say whether a Markdown index of the documentation for AI crawlers (`llms.txt`) helps an agent: that is tested, not assumed, by roadmap item A14.

Next: [the guide's index](README.md), and [WORKFLOWS.md](../WORKFLOWS.md) for ways of working with these tools in a team.
