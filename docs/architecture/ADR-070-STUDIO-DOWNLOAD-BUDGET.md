# ADR-070: What Forge Studio's Download Budget Is For

**Status:** Accepted
**Date:** 2026-11-21
**Decided by:** the maintainer (Studio is a demo for FP&A and tech people and investors in Canadian and US cities; showcase features without reinventing the wheel, as long as it loads in time); the analysis and the numbers are Claude's
**Changes:** the size check in `.github/workflows/ci.yml`; part of the context of ADR-069

---

## Context

Forge Studio (ADR-056) calculates in the browser with one WebAssembly module, `forge_wasm_bg.wasm`, which is most of what a visitor downloads on a first visit. Since PR #208 (2026-11-14) CI has failed when the module, gzipped as GitHub Pages serves it, is over a set size: 2,000,000 bytes at first, then 1,362,000 from 2026-11-20 (ADR-069: the size that day plus 100 KB). Neither number said what it was for. With no target behind it, the budget decided architecture by itself:

- ADR-069 replaced the `jsonschema` crate with Forge's own schema reader. The module went from 1,939,940 to 1,261,203 bytes gzipped, and the reader's patterns got a matcher of their own because `regex` alone was 277 KB.
- PR #323 used `regex` once in the importer (to find a range of a cell's own column), which added 288 KB and failed the budget. The use was replaced by hand-written code that was no better than the library, only smaller.

Studio is a demo. The people it is for are FP&A analysts, engineers and investors in Toronto, Montreal and Vancouver, the Bay Area, Seattle, Boston and New York, opening a link from a post, an email or a pitch, on a recent phone or laptop. Their usual connection is fast: Ookla's 2025 and 2026 reports put the median mobile download in large Canadian cities at about 115 to 145 Mbps (Vancouver 145 Mbps in H1 2026) and fixed fibre medians at about 300 to 375 Mbps in both countries. The connection that loses a first impression is the bad one that is still common in those places: conference or hotel Wi-Fi, an airport, a meeting room in a basement, a corporate proxy that inspects downloads. This ADR takes that as **10 Mbps**.

## Decision

1. **The target.** On a first visit, Forge Studio is usable within **3 seconds on a 10 Mbps connection**. Return visits use the offline cache until the next release.
2. **The ceiling that follows.** 10 Mbps moves about 1.25 MB a second; with about half a second for connections and Studio's other files, the module may be **3,000,000 bytes gzipped**. CI fails above that. On a city 5G or office fibre connection that is a fraction of a second.
3. **Growth is a decision, not a failure.** Under the ceiling, a pull request that grows the module by more than 100 KB gzipped says in its description and in the CHANGELOG what the growth buys. CI prints the size on every run.
4. **A stable library before code of our own.** Under the ceiling, size alone is never a reason to replace a well-maintained library with hand-written code. Code of our own needs its own merits: it is more correct, simpler, or does what the library cannot. When the ceiling is near, first build the same library smaller (fewer features, the `wasm` profile's settings), then consider a smaller library, and only then write our own.
5. **Unicode.** Forge's names are Unicode: `hypothèses.coût_unitaire` and `假设.单价` calculate, and survive `forge export` and `forge import` (checked 2026-11-21). Anything that matches names must match Unicode letters. `regex` does; `regex-lite`, its smaller sibling, matches only ASCII letters, so it is not a smaller substitute for Forge.
6. **When to look again.** If the audience changes (a market with slower connections, a product rather than a demo), or Studio's other files become a large share of the download, the target is decided again, here.

## Consequences

- `regex` may be used where it is the right tool, in the browser too, as long as the module stays under the ceiling.
- ADR-069's main reason in its context, the 684 KB `jsonschema` cost against a 2,000,000-byte budget, no longer holds as a constraint: 1.29 MB plus 0.68 MB is under 3 MB. Its other reasons (one validator with the same messages on every surface, the crates it brought) still stand. Whether to keep Forge's reader or return to `jsonschema` is a question of its own (roadmap T1-REVISIT-VALIDATOR), decided on those merits; until then ADR-069 stands, and its reader keeps being checked against `jsonschema` on every test run.
- The 3 seconds are a download estimate. Compiling the module on a phone adds to it and has not been measured; the target includes it, so a measurement on a mid-range phone belongs with the next large growth.

## Alternatives considered

- **Keep 1,362,000 (the size of 2026-11-20 plus 100 KB).** A tripwire with no target pushes toward custom code for size alone, which is what happened in PR #323.
- **2 seconds on 10 Mbps (about 1.9 MB).** Possible, close to the module with `jsonschema` in it, but it leaves little room for the next library.
- **5 seconds on weak mobile, 1.6 Mbps (about 1.0 MB).** Below today's module; it would force more code of our own before anything else.
- **No budget, the size only reported.** Nothing would stop the slow, unnoticed growth that a budget exists to catch.
