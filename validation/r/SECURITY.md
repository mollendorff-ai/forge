# Security Policy

## Supported versions

forge-e2e-r is a test suite for [Forge](https://github.com/mollendorff-ai/forge), not software to deploy. It has no releases of its own. Only the latest commit on `main` is maintained.

## Reporting a vulnerability

Please do not open a public issue for a security problem.

Email **security@mollendorff.ai** with:

- what you found and what it lets someone do,
- steps or a test spec that reproduce it,
- the commit you tested, your operating system and your R version.

You can also use GitHub's private vulnerability reporting, under the **Security** tab of this repository, if it is enabled.

This is a small project kept up on a best-effort basis. I aim to reply within 14 days, and I will tell you what I plan to do about the report. Please give me reasonable time to ship a fix before you share the details publicly. I am glad to credit you if you want that.

## What is in scope

- The Rust runner: how it reads the YAML test specs and how it starts the `forge` binary (`FORGE_BIN`) and R with values taken from them.
- The R validators in `validators/` and the shell script that runs them.
- How Forge's Validation workflow, which runs this suite, checks out and runs this repository's code (it uses the branch with the same name as the Forge branch). Report problems in the workflow itself in the Forge repository.

## What is not in scope

- Vulnerabilities in Forge itself: report those in the [Forge repository](https://github.com/mollendorff-ai/forge/security/policy).
- Vulnerabilities in R or in the R packages the validators use: report those to their maintainers.
- A spec that fails, or a result that disagrees with R, is a bug or a finding about Forge, not a security problem: please open a normal issue for those.
