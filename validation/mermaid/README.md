# Every Mermaid diagram renders

GitHub draws a ` ```mermaid ` block that does not parse as an error box, and says nothing else. `check.mjs` renders every such block of every Markdown file git tracks, in Chromium, with mermaid at the exact version in `package.json` (GitHub's renderer follows the latest release closely), and fails naming the file, the line and the block. Roadmap item T1-MERMAID-CHECK.

Two mistakes it catches that were made writing `docs/architecture`: `graph` as a node id (a reserved word), and a `;` inside a message of a sequence diagram (it ends the message).

## Running it

```bash
cd validation/mermaid
npm install                       # mermaid 12.3.0, Playwright 1.63.0
npx playwright install chromium   # not needed where Chromium is installed already
node check.mjs                    # every Markdown file of the repository
node check.mjs ../../README.md    # some files
node check.mjs --out /tmp/d       # and a PNG of each diagram
```

The `Diagrams` workflow (`.github/workflows/diagrams.yml`) runs it on every pull request and push to main that changes a Markdown file, after checking that a diagram that does not parse fails; the PNGs are the run's `diagrams` artifact.

Moving to a newer mermaid: change its version in `package.json`, and in this file; every diagram on main must still render.
