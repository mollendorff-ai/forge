// Every Mermaid diagram of the repository's Markdown renders (roadmap
// T1-MERMAID-CHECK).
//
// GitHub draws a ```mermaid block that does not parse as an error box, and
// says nothing else: two diagrams of docs/architecture did not parse when they
// were written ("graph" is a reserved word as a node id; a ";" ends a message
// of a sequence diagram). This renders each block in Chromium with mermaid at
// the exact version of package.json (GitHub's renderer follows the latest
// release closely), and fails naming the file, the line and the block.
//
//   node validation/mermaid/check.mjs [--out <dir>] [file.md ...]
//
// With no file, every Markdown file git tracks. With --out, each diagram is
// also saved as a PNG (<file>-<block>.png), for a reviewer to look at.

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync } from "node:fs";
import { createRequire } from "node:module";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "..", "..");
const require = createRequire(import.meta.url);
const mermaidScript = join(dirname(require.resolve("mermaid/package.json")), "dist", "mermaid.min.js");
const mermaidVersion = JSON.parse(readFileSync(join(dirname(mermaidScript), "..", "package.json"), "utf8")).version;

/** The ```mermaid blocks of a Markdown text: the line each starts on and its source. */
export function mermaidBlocks(text) {
  const blocks = [];
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    const open = /^(\s*)(`{3,}|~{3,})\s*mermaid\s*$/.exec(lines[i]);
    if (!open) continue;
    const [, indent, fence] = open;
    const body = [];
    let j = i + 1;
    // A fence closes on the same character, at least as long (CommonMark)
    while (j < lines.length && !new RegExp(`^\\s*${fence[0]}{${fence.length},}\\s*$`).test(lines[j])) {
      body.push(lines[j].startsWith(indent) ? lines[j].slice(indent.length) : lines[j].trimStart());
      j++;
    }
    blocks.push({ line: i + 1, source: body.join("\n") });
    i = j;
  }
  return blocks;
}

function markdownFiles(args) {
  if (args.length) return args.map((f) => resolve(f));
  const listed = execFileSync("git", ["ls-files", "-z", "--", "*.md"], { cwd: repo, encoding: "utf8" });
  return listed.split("\0").filter(Boolean).map((f) => join(repo, f));
}

const args = process.argv.slice(2);
const outAt = args.indexOf("--out");
const out = outAt >= 0 ? resolve(args.splice(outAt, 2)[1]) : null;
if (out) mkdirSync(out, { recursive: true });

const diagrams = markdownFiles(args).flatMap((file) =>
  mermaidBlocks(readFileSync(file, "utf8")).map((b, n) => ({ ...b, file, block: n + 1 })),
);

const browser = await chromium.launch();
const page = await browser.newPage({ deviceScaleFactor: out ? 2 : 1 });
await page.setContent("<!doctype html><html><body></body></html>");
await page.addScriptTag({ path: mermaidScript });
await page.evaluate(() => window.mermaid.initialize({ startOnLoad: false, securityLevel: "strict" }));

const failures = [];
for (const d of diagrams) {
  const name = relative(repo, d.file).startsWith("..") ? d.file : relative(repo, d.file);
  const where = `${name}:${d.line}`;
  const error = await page.evaluate(async ({ source, id }) => {
    try {
      // parse throws on what GitHub shows as an error box; render on the rest
      // (a diagram that parses but cannot be laid out)
      await window.mermaid.parse(source);
      const { svg } = await window.mermaid.render(id, source);
      document.body.innerHTML = svg;
      return null;
    } catch (e) {
      document.querySelectorAll(`[id^="${id}"]`).forEach((n) => n.remove());
      return String(e?.message ?? e);
    }
  }, { source: d.source, id: `diagram-${d.block}` });
  if (error) {
    failures.push({ ...d, where, error });
    console.log(`FAIL ${where} (block ${d.block})\n${error.replace(/^/gm, "     ")}`);
    if (process.env.GITHUB_ACTIONS) {
      const message = `Mermaid block ${d.block} does not render: ${error}`.replace(/%/g, "%25").replace(/\r?\n/g, "%0A");
      console.log(`::error file=${name},line=${d.line}::${message}`);
    }
  } else {
    console.log(`ok   ${where} (block ${d.block})`);
    if (out) {
      const png = `${name.replace(/[\\/]/g, "_")}-${d.block}.png`;
      await page.locator("body > svg").screenshot({ path: join(out, png) });
    }
  }
}
await browser.close();

const files = new Set(diagrams.map((d) => d.file)).size;
console.log(`\n${diagrams.length - failures.length} of ${diagrams.length} diagrams in ${files} files render (mermaid ${mermaidVersion})`);
if (failures.length) {
  console.log(`${failures.length} do not: ${failures.map((f) => `${f.where} (block ${f.block})`).join(", ")}`);
  process.exit(1);
}
