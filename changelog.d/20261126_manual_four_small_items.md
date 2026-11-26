### Changed

- **The docs say `forge edit` changes a column of text.** docs/MCP.md, docs/FEATURES.md and the agent skill give the column of text beside the numbers and dates, with `--set 'products.name=[Basic, Plus Pro]'`, and say a column of true and false, or with an empty cell, is still refused.
- **The docs show every MCP tool over REST.** docs/FEATURES.md lists `POST /api/v1/<tool>` and `GET /api/v1/openapi.json` with a worked call (goal seek on `examples/starter.yaml`), and docs/MCP.md says the tools are also reachable over HTTP.
- **The README carries the MCP Registry name** (`mcp-name: io.github.mollendorff-ai/forge`) as visible text, so the registry can verify the crate once the next release is on crates.io; a test holds it equal to `server.json`'s name.
- **The acquisition example says how its 91.73 is worked out.** The command prints only the best path, so the page shows the arithmetic for the 40% bid and the 69% threshold, which `src/cli/commands/prediction/decision_tree.rs` asserts.
