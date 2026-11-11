// Editing a formula on its card (roadmap S7), in a headless browser:
//
// - on every example set, every formula (scalars, table columns, included
//   files) is changed on its card from =x to =(x): the file then differs from
//   before in that one line and nowhere else, byte for byte, and Forge reads
//   back exactly the new formula;
// - an invalid formula is refused with Forge's message, and the file is the
//   same, byte for byte.
//
//   node studio/tests/formulas.mjs <built Studio dir or address>

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";
import { SETS } from "../build.mjs";

const [where] = process.argv.slice(2);
const remote = /^https?:/.test(where);
const dir = resolve(where);
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".wasm": "application/wasm" };
const server = createServer((req, res) => {
  let file = join(dir, decodeURIComponent(new URL(req.url, "http://x").pathname));
  if (existsSync(file) && statSync(file).isDirectory()) file = join(file, "index.html");
  if (!file.startsWith(dir) || !existsSync(file)) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { "content-type": TYPES[extname(file)] || "application/octet-stream" });
  res.end(readFileSync(file));
});
if (!remote) await new Promise((r) => server.listen(0, "127.0.0.1", r));
const base = remote ? where.replace(/\/$/, "") : `http://127.0.0.1:${server.address().port}`;

const browser = await chromium.launch();
const failures = [];
const fail = (m) => failures.push(m);
// The page's files: in its storage once something changed, else the example's
const files = (page) => page.evaluate(async () => {
  const saved = JSON.parse(localStorage.getItem("forge-studio-v3") || "null");
  if (saved) return saved.files;
  const id = new URLSearchParams(location.search).get("example");
  return (await (await fetch("examples.json")).json()).find((x) => x.id === id).files;
});
const forge = (page, tool, request) => page.evaluate(async ([t, r]) => {
  const { call } = await import("./forge.js");
  try { return { ok: await call(t, r) }; } catch (e) { return { error: e.message }; }
}, [tool, request]);
const generation = (page) => page.evaluate(() => Number(document.body.dataset.generation || 0));

function formulasOf(structure) {
  const out = [];
  const scalars = [...structure.scalars, ...structure.includes.flatMap((i) => i.scalars)];
  for (const s of scalars) if (s.formula) out.push({ path: s.path, formula: s.formula, column: false });
  for (const t of [...structure.tables, ...structure.includes.flatMap((i) => i.tables)]) {
    for (const c of t.columns) if (c.formula) out.push({ path: `${t.name}.${c.name}`, formula: c.formula, column: true });
  }
  return out;
}
const fileOf = (structure, main, path) => {
  const m = /^@([^.]+)\./.exec(path);
  return m ? structure.includes.find((i) => i.as === m[1]).file : main;
};
const formulaIn = (structure, path) => formulasOf(structure).find((f) => f.path === path)?.formula;

// =x became =(x): the line must be the old one with "(" and ")" added, so
// the key, the quotes, a comment and the rest of a flow mapping are kept.
function onlyParentheses(before, after) {
  if (after.length !== before.length + 2) return false;
  let p = 0;
  while (p < before.length && before[p] === after[p]) p++;
  let q = 0;
  while (q < before.length - p && before[before.length - 1 - q] === after[after.length - 1 - q]) q++;
  return after[p] === "(" && after[after.length - 1 - q] === ")" && after.slice(p + 1, after.length - 1 - q) === before.slice(p, before.length - q);
}

async function openEditor(page, f) {
  if (f.column) {
    const col = page.locator(`[data-col="${f.path}"]`);
    if ((await col.getAttribute("aria-expanded")) !== "true") await col.click();
  }
  await page.click(`[data-edit="${f.path}"]`);
  return page.locator(`[data-finput="${f.path}"]`);
}

let edited = 0;
for (const set of SETS) {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  page.on("pageerror", (e) => fail(`${set.id}: ${e.message}`));
  await page.goto(`${base}/?example=${set.id}`);
  await page.waitForFunction(() => document.body.dataset.state !== "loading", null, { timeout: 120000 });
  const start = await files(page);
  const structure = (await forge(page, "model", { files: start, main: set.main })).ok;
  const list = formulasOf(structure);
  console.log(`${set.id}: ${list.length} formulas`);
  for (const f of list) {
    const t0 = Date.now();
    const file = fileOf(structure, set.main, f.path);
    const before = (await files(page))[file];
    const formula = `=(${f.formula.slice(1)})`;
    const input = await openEditor(page, f);
    await input.fill(formula);
    const g = await generation(page);
    await input.press("Enter");
    await page.waitForFunction((x) => Number(document.body.dataset.generation) > x || !!document.querySelector("[data-fmsg]:not([hidden])"), g, { timeout: 60000 });
    const shown = page.locator("[data-fmsg]:not([hidden])");
    const message = (await shown.count()) ? await shown.textContent() : null;
    if (message) { fail(`${set.id} ${f.path}: ${formula} was refused: ${message}`); await page.click(`[data-fcancel="${f.path}"]`); continue; }
    await page.waitForFunction(() => document.body.dataset.state !== "loading");
    const now = await files(page);
    const a = before.split("\n"), b = now[file].split("\n");
    const changed = b.map((l, i) => (l !== a[i] ? i : -1)).filter((i) => i >= 0);
    if (a.length !== b.length || changed.length !== 1) fail(`${set.id} ${f.path}: ${changed.length} lines of ${file} changed (${a.length} lines before, ${b.length} after)`);
    else if (!onlyParentheses(a[changed[0]], b[changed[0]])) fail(`${set.id} ${f.path}: the line changed beyond the formula:\n    before: ${a[changed[0]]}\n    after:  ${b[changed[0]]}`);
    for (const other of Object.keys(now)) if (other !== file && now[other] !== (await files(page))[other]) fail(`${set.id} ${f.path}: ${other} changed too`);
    const read = (await forge(page, "model", { files: now, main: set.main })).ok;
    if (formulaIn(read, f.path) !== formula) fail(`${set.id} ${f.path}: Forge reads ${formulaIn(read, f.path)}, not ${formula}`);
    edited++;
    if (process.env.VERBOSE) console.log(`  ${f.path} ${Date.now() - t0} ms`);
  }
  await page.close();
}

// Refused: the file does not change and the message is Forge's
{
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await page.goto(`${base}/?example=bagel`);
  await page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });
  for (const bad of ["=SUM(months.profit", "=NOPE(1)", "=nothing_here + 1", "=margin * 2"]) {
    const before = await files(page);
    const input = await openEditor(page, { path: "year.margin", column: false });
    await input.fill(bad);
    await input.press("Enter");
    const shown = await page.locator("[data-fmsg]:not([hidden])").textContent({ timeout: 30000 });
    const after = await files(page);
    if (JSON.stringify(after) !== JSON.stringify(before)) fail(`${bad}: the files changed although Forge refused it`);
    const candidate = await page.evaluate(async ([text, f]) => (await import("./text.js")).setFormulaText(text, "year", "margin", f), [before["bagel.yaml"], bad]);
    const forgeSays = await forge(page, "validate", { files: { ...before, "bagel.yaml": candidate }, main: "bagel.yaml" });
    if (!forgeSays.error) fail(`${bad}: Forge accepts it; choose another invalid formula for this test`);
    else if (shown !== forgeSays.error) fail(`${bad}: the card says "${shown}", Forge says "${forgeSays.error}"`);
    await page.click('[data-fcancel="year.margin"]');
  }
  await page.close();
}

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Formulas on cards: ${edited} formulas in ${SETS.length} example sets changed on their cards, each changing its one line and nothing else; 4 invalid formulas refused with Forge's message and no change.`);
