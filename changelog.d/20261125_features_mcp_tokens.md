### Changed

- **The MCP tool list is 8.6% shorter** (roadmap T3-MCP-TOKENS): 27,661 bytes where it was 30,262 (measured as the JSON of `tools/list`, which every client reads at the start of a session). The `includes` field's doc, repeated in 17 tools, is one line; `forge_edit`'s description no longer lists the ops its schema already describes; `forge_game`'s is shorter. The 23 tools, their names and their parameters are unchanged.
