# ADR-081: Every Tool over REST, Described by an OpenAPI Document Made from It

**Status:** Accepted (2026-11-25); built: `POST /api/v1/<tool>` for every MCP tool and `GET /api/v1/openapi.json` (roadmap T3-API)
**Date:** 2026-11-25
**Decided by:** the maintainer (2026-11-25), who accepted the five decisions as written, the 422 for a tool's failure and the five endpoints left as they are among them; the design and the analysis are Claude's
**For:** ADR-047 (one core function under every surface), ADR-049 (inline content). It changes none of their decisions

---

## Context

`forge serve` answered five operations: `validate`, `calculate`, `audit`, `export` and `import`. The MCP server has 23 tools, so a system that could not speak MCP or run a command had none of the analyses: sensitivity, goal seek, break-even, variance, scenarios and their comparison, the tornado, simulation, decision trees, real options, bootstrap, Bayesian networks, games, `edit`, and the discovery tools. Roadmap T3-API asked for an endpoint for each, sharing its core function with the MCP tool, and an OpenAPI document generated from the handlers, not written by hand.

Two things were already in place. Each MCP tool reads a request type that derives `JsonSchema` (schemars 1.2, through rmcp), and `tools/list` gives that schema to every client. And each tool's method calls the command's core function. What was missing was a way to run a tool without an MCP session: rmcp's router calls a tool only with a `RequestContext`, whose `Peer` only rmcp can build.

## Decision

1. **Every MCP tool has an endpoint.** A tool without one of its own is served at `POST /api/v1/<name>`, its name as the command's: `forge_goal_seek` at `/api/v1/goal-seek`, `forge_decision_tree` at `/api/v1/decision-tree`. The routes are made from the tool list when the server starts, so a new tool gets its endpoint, its line in `GET /`, `/version`'s `features` and its path in the document with nothing else to write.
2. **An endpoint runs the tool itself.** `ForgeMcpServer::call(name, arguments)` reads the tool's request from the JSON body as `tools/call` reads it, and runs the tool's method, which calls the command's core function. The REST surface and the MCP surface share the request type, the method and the core: an endpoint cannot answer something else than its tool. A test runs every served tool both ways and compares the answers (`every_tool_has_an_endpoint_that_answers_what_the_tool_answers`), the time a run took aside. The server's `--root` and token hold for every endpoint, as for the five.
3. **The answer is the tool's JSON.** `200 {success: true, request_id, data}`, `data` being what the tool answers. A tool that fails, or arguments that are not its request, is `422 {success: false, request_id, error}` with the tool's message, as MCP answers `isError` with it. A body that is not JSON is `400` in the same shape. No body is no arguments, for the tools that take none (`functions`).
4. **The five endpoints of their own stay as they are.** `validate`, `calculate`, `audit`, `export` and `import` keep their requests and their answers (`valid`, `calculated`, ... in `data`), since clients use them. They call the same core functions as their tools already (T1-INTERFACE-PARITY), but their answers are shaped differently from their tools' JSON. Making them answer as their tools do would change what those clients read; it is left for a version that can break them.
5. **The OpenAPI document is made from the same schemas.** `GET /api/v1/openapi.json` answers an OpenAPI 3.1 document built when the server starts. Each tool's body is the schema `tools/list` gives the tool, its `$defs` moved to `components.schemas` and its references with them. The five endpoints' bodies are schemars' schemas of their request types, which now derive `JsonSchema`. OpenAPI 3.1's schema objects are JSON Schema 2020-12, which is what schemars writes, so no schema is translated. A test checks that every reference resolves, that every schema is valid JSON Schema 2020-12, that the operation ids are unique, and that each tool's body is its listed schema. On 2026-11-25 the document was checked against the OpenAPI 3.1 schema (`src/schemas/validation/schema.yaml` of the specification's `v3.1-dev` branch) with Python's `jsonschema` 4.26: no error, 27 paths, 66 references resolved.

## Alternatives

- **aide**, the library that writes OpenAPI from axum handlers with schemars: its 0.15.1 needs schemars 0.9, and rmcp uses 1.2. Two versions of schemars would be a duplicate this project does not allow for a library that has a newer one.
- **utoipa** (6.0): its own `ToSchema` derive on every request, beside the `JsonSchema` that MCP needs, gives two schemas of each request that can say different things, and its typed schema model would have to read back schemars' output. The document's own part is a handful of OpenAPI keys around schemas that already exist; writing them is less than keeping two derives in step.
- **One route `/api/v1/tools/{name}`**: shorter, but a client or a generated SDK then has one operation with 18 bodies, and the document cannot say which body goes with which tool.
- **Calling rmcp's router**: it needs a `RequestContext` whose `Peer` only rmcp can construct. `ForgeMcpServer::call` is a `match` over the 23 names; a test calls each tool through it, and a tool missing from it fails that test.

## Left out

- **Typed answers.** The document says each answer's envelope (`Answer`, `Failure`) and not the shape of `data`: the MCP tools declare no output schema either. Typing them means an `outputSchema` for each tool first, which MCP would then use too.
- **forge-wasm.** Forge Studio calls the core functions in the browser (ADR-055); it has no HTTP server.
- **Long runs.** A large simulation holds its request open until it ends; jobs that are started and then polled are for a user who needs them.
- **GET for the discovery tools.** `schema`, `functions`, `templates` and `examples` are POST like the others, so every tool has one shape.

## Consequences

- A system that speaks HTTP has every analysis Forge has, with the same answers as an agent over MCP.
- A new MCP tool needs no REST work and no document work; the tests that compare the two surfaces and check the document include it at once.
- The architecture pages that describe the API server (08, 01, `EMBEDDING.md`) and the guide's FEATURES page still say five operations: they are filed for the architecture and manual lanes.
