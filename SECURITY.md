# Security Policy

## Supported versions

Forge is a release candidate on the way to a stable 10.0.0. Only the latest release gets security fixes.

## Reporting a vulnerability

Please do not open a public issue for a security problem.

Email **security@mollendorff.ai** with:

- what you found and what it lets someone do,
- steps or a model file that reproduce it,
- the Forge version (`forge --version`) and your operating system.

You can also use GitHub's private vulnerability reporting, under the **Security** tab of this repository, if it is enabled.

Forge is a small project kept up on a best-effort basis. I aim to reply within 14 days, and I will tell you what I plan to do about the report. Please give me reasonable time to ship a fix before you share the details publicly. I am glad to credit you in the release notes if you want that.

## Files the servers can reach

`forge mcp` and `forge serve` read and write the files a request names, as the user who runs them. An agent, or a prompt injected into one, can name any file that user can reach. With `--root DIR`, a path is resolved (`..` and symbolic links followed) and refused unless it is inside `DIR`, for reading and for writing; a model that includes a file outside `DIR` is refused before that file is opened. See [docs/MCP.md](docs/MCP.md#keeping-files-inside-one-folder). The check runs before each command: it limits what requests name, and is not a sandbox against a process that changes links inside `DIR` at the same time. A way past it that needs no such process is in scope.

## What is in scope

- The `forge` command and library: reading YAML models, including `_includes` and `@` references to other files.
- `forge import` and `forge export` on Excel files.
- The MCP server (`forge mcp`), which takes YAML sent by a client, and its `--root` check.
- The HTTP REST API (`forge serve`): its token, its `Host` header checks, its `--root` check and what it does with the models it receives.
- `forge update`, which downloads and installs a release from GitHub.

Wrong results from a calculation are bugs, not security problems: please open a normal issue for those.
