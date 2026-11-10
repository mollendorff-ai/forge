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

## What is in scope

- The `forge` command and library: reading YAML models, including `_includes` and `@` references to other files.
- `forge import` and `forge export` on Excel files.
- The MCP server (`forge mcp`), which takes YAML sent by a client.
- The HTTP REST API (`forge serve`): its token, its `Host` header checks and what it does with the models it receives.
- `forge update`, which downloads and installs a release from GitHub.

Wrong results from a calculation are bugs, not security problems: please open a normal issue for those.
