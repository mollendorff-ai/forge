// Open every example set in Forge Studio in a headless browser and check that
// every scalar on screen is the number `forge calculate` gives natively for
// that model (roadmap S3). The native numbers are forge-wasm's expected
// results (cargo run -p forge-wasm --example expected).
//
//   node studio/tests/screen.mjs <built Studio dir> <expected.json>

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";
import { SETS } from "../build.mjs";

// The first argument may also be the address of a running server.
const [where, expectedPath] = process.argv.slice(2);
const dir = resolve(where);
const expected = JSON.parse(readFileSync(resolve(expectedPath), "utf8"));
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".wasm": "application/wasm" };
const RELATIVE = 1e-9;
const CASES = JSON.parse(readFileSync(new URL("../../forge-wasm/tests/cases.json", import.meta.url), "utf8"));

const server = createServer((req, res) => {
  let file = join(dir, decodeURIComponent(new URL(req.url, "http://x").pathname));
  if (existsSync(file) && statSync(file).isDirectory()) file = join(file, "index.html");
  if (!file.startsWith(dir) || !existsSync(file)) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { "content-type": TYPES[extname(file)] || "application/octet-stream" });
  res.end(readFileSync(file));
});
const remote = /^https?:/.test(where);
if (!remote) await new Promise((r) => server.listen(0, "127.0.0.1", r));
const base = remote ? where.replace(/\/$/, "") : `http://127.0.0.1:${server.address().port}`;

const browser = await chromium.launch();
const failures = [];
const close = (a, b) => Math.abs(a - b) <= RELATIVE * Math.max(1, Math.abs(a), Math.abs(b));

async function screen(page) {
  await page.waitForFunction(() => document.body.dataset.state !== "loading", null, { timeout: 120000 });
  const state = await page.evaluate(() => document.body.dataset.state);
  const values = await page.$$eval("[data-scalar]", (els) => els.map((e) => [e.dataset.scalar, e.dataset.value]));
  return { state, values };
}
function check(label, shown, native) {
  const seen = new Map(shown.values);
  if (shown.state !== "ready") failures.push(`${label}: the page says ${shown.state}`);
  for (const [path, value] of Object.entries(native)) {
    if (!seen.has(path)) { failures.push(`${label}: ${path} is not on screen`); continue; }
    const text = seen.get(path);
    // A date (ADR-060) is its text, shown exactly; a number within the rounding
    const differs = value === null ? text !== "" : typeof value === "string" ? text !== value : !close(Number(text), value);
    if (differs) failures.push(`${label}: ${path} shows ${text}, forge calculate gives ${value}`);
  }
  for (const path of seen.keys()) if (!(path in native)) failures.push(`${label}: ${path} is on screen but not in forge calculate's answer`);
  return seen.size;
}

const rows = [];
for (const set of SETS) {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  const errors = [];
  page.on("pageerror", (e) => errors.push(e.message));
  page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });
  const started = performance.now();
  await page.goto(`${base}/?example=${set.id}`);
  const shown = await screen(page);
  const ms = performance.now() - started;
  const native = expected[set.case] && expected[set.case].ok;
  if (!native) failures.push(`${set.id}: no native answer for "${set.case}"`);
  const count = native ? check(set.id, shown, native.scalars) : 0;
  rows.push([set.id, count, ms]);

  // A scenario on screen is forge calculate --scenario
  for (const c of CASES.filter((c) => c.name.startsWith(`${set.case}, `))) {
    const before = Number(await page.evaluate(() => document.body.dataset.generation));
    await page.click(`[data-scn="${c.args.scenario}"]`);
    await page.waitForFunction((g) => Number(document.body.dataset.generation) > g && document.body.dataset.state !== "loading", before);
    check(`${set.id} (${c.args.scenario})`, await screen(page), expected[c.name].ok.scalars);
  }
  for (const e of errors) failures.push(`${set.id}: browser error: ${e}`);
  await page.close();
}
await browser.close();
if (!remote) server.close();

const summary = ["## Forge Studio on screen", "", "| Example | Scalars on screen | Time to first numbers (ms) |", "| --- | ---: | ---: |",
  ...rows.map(([id, n, ms]) => `| ${id} | ${n} | ${ms.toFixed(0)} |`)].join("\n");
console.log(summary);
if (process.env.GITHUB_STEP_SUMMARY) {
  const { appendFileSync } = await import("node:fs");
  appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary + "\n");
}
if (failures.length) {
  console.error(`\n${failures.length} problems:\n` + failures.slice(0, 60).join("\n"));
  process.exit(1);
}
console.log("\nEvery scalar on screen is forge calculate's.");
