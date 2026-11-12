// Suggestions while a formula is typed (roadmap S10), in a headless browser
// with Forge's own answers:
//
// - for every formula of every example, typing the first three letters of each
//   name or function in it (all of it when shorter) offers that name or
//   function, written as the formula writes it, and taking the suggestion
//   gives the same word;
// - the functions offered are Forge's list (169) and only those: a function
//   Forge does not list is never offered;
// - nothing is offered inside a "text" or for a number;
// - on the cards, the strip above the formula field offers a name, a tap
//   inserts exactly its text and the formula is then saved as typed.
//
//   node studio/tests/suggest.mjs <built Studio dir or address>

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";

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
const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
page.on("pageerror", (e) => fail(e.message));
await page.goto(`${base}/`);
await page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });

// Every formula of every example, with what Forge says about the model, and
// the suggestions for the first letters of each word in it
const result = await page.evaluate(async () => {
  const { call } = await import("./forge.js");
  const { scopeOf, namesFor, functionsOf, suggest, apply } = await import("./suggest.js");
  const sets = await (await fetch("examples.json")).json();
  const answer = await call("functions", {});
  const functions = functionsOf(answer);
  const listed = new Set(functions.map((f) => f.name));
  const problems = [];
  let formulas = 0, words = 0, offered = 0;
  const wordsOf = (formula) => {
    const plain = formula.replace(/"(?:[^"\\]|\\.)*"/g, '""');
    return [...plain.matchAll(/(?<![A-Za-z0-9_.@])(@?[A-Za-z_][A-Za-z0-9_.@]*)(\()?/g)].map((m) => ({ word: m[1], call: !!m[2] }));
  };
  for (const set of sets) {
    const model = await call("model", { files: set.files, main: set.main });
    const entries = [];
    const collect = (part) => {
      for (const s of part.scalars) if (s.formula != null) entries.push({ path: s.path, formula: s.formula });
      for (const t of part.tables) for (const c of t.columns) if (c.formula) entries.push({ path: `${t.name}.${c.name}`, formula: c.formula });
    };
    collect(model);
    model.includes.forEach(collect);
    for (const { path, formula } of entries) {
      formulas++;
      const scope = scopeOf(model, path);
      const index = { names: namesFor(model, scope), functions };
      for (const { word, call: isCall } of wordsOf(formula)) {
        words++;
        const typed = word.slice(0, Math.min(3, word.length));
        const list = suggest(index, typed, typed.length);
        const want = isCall ? word + "(" : word;
        const hit = list.find((s) => s.label === want || (!isCall && s.insert === word));
        if (!hit) { problems.push(`${set.id}: ${path}: typing "${typed}" does not offer ${want}`); continue; }
        offered++;
        const after = apply(typed, hit).text;
        if (after !== want) problems.push(`${set.id}: ${path}: taking ${hit.label} after "${typed}" gives "${after}", not "${want}"`);
        if (isCall && !listed.has(word)) problems.push(`${set.id}: ${path}: ${word} is not in Forge's function list`);
      }
    }
  }
  // Only Forge's functions are offered, nothing in a text or a number
  const scope0 = { ns: null, group: null, table: null };
  const probe = { names: [], functions };
  const offeredFns = new Set();
  for (const a of "ABCDEFGHIJKLMNOPQRSTUVWXYZ") for (const s of suggest(probe, a, 1)) if (s.kind === "function") offeredFns.add(s.insert.replace("(", ""));
  for (const n of offeredFns) if (!listed.has(n)) problems.push(`${n} is offered but not in Forge's list`);
  if (suggest(probe, "ZZNOPE", 6).length) problems.push("a function Forge does not list was offered");
  if (suggest(probe, '="SU', 4).length) problems.push("something was offered inside a text");
  if (suggest(probe, "=12", 3).length || suggest(probe, "=1.5", 4).length) problems.push("something was offered for a number");
  return { problems, formulas, words, offered, total: answer.total, count: functions.length };
});
for (const p of result.problems) fail(p);
if (result.count !== result.total) fail(`the function list has ${result.count} entries, Forge says ${result.total}`);

// On the cards
await page.goto(`${base}/?example=bagel`);
await page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });
await page.click('[data-edit="year.margin"]');
const field = page.locator('[data-finput="year.margin"]');
await field.fill("=profit / re");
await field.evaluate((el) => el.setSelectionRange(el.value.length, el.value.length));
await field.dispatchEvent("input");
const strip = page.locator('[data-fslot="year.margin"] [data-suggest]');
const labels = await strip.locator("[data-sug]").allTextContents();
if (!labels.includes("revenue")) fail(`typing "re" in year.margin offers ${labels.join(", ") || "nothing"}, not revenue`);
await strip.locator("[data-sug]", { hasText: /^revenue$/ }).click();
if ((await field.inputValue()) !== "=profit / revenue") fail(`taking revenue gave "${await field.inputValue()}"`);
await field.fill("=profit / revenue * assumptions.pr");
await field.evaluate((el) => el.setSelectionRange(el.value.length, el.value.length));
await field.dispatchEvent("input");
const paths = await strip.locator("[data-sug]").allTextContents();
if (!paths.includes("assumptions.price_per_dozen")) fail(`typing "assumptions.pr" offers ${paths.join(", ") || "nothing"}, not assumptions.price_per_dozen`);
await field.fill("=SU");
await field.evaluate((el) => el.setSelectionRange(el.value.length, el.value.length));
await field.dispatchEvent("input");
await strip.locator("[data-sug]", { hasText: /^SUM\($/ }).waitFor({ timeout: 30000 }).catch(() => {});
const fns = await strip.locator("[data-sug]").allTextContents();
if (!fns.includes("SUM(")) fail(`typing "SU" offers ${fns.join(", ") || "nothing"}, not SUM(`);
await strip.locator("[data-sug]", { hasText: /^SUM\($/ }).click();
if ((await field.inputValue()) !== "=SUM(") fail(`taking SUM( gave "${await field.inputValue()}"`);
// A formula built with the strip is saved as typed
await field.fill("=profit / re");
await field.evaluate((el) => el.setSelectionRange(el.value.length, el.value.length));
await field.dispatchEvent("input");
await strip.locator("[data-sug]", { hasText: /^revenue$/ }).click();
await field.fill((await field.inputValue()) + " * 2");
await page.click('[data-fsave="year.margin"]');
const kept = await page.waitForFunction(() => {
  try { return JSON.parse(localStorage.getItem("forge-studio-v3")).files["bagel.yaml"]; } catch (e) { return null; }
}, null, { timeout: 120000 }).then((h) => h.jsonValue(), () => "");
if (!/formula: "=profit \/ revenue \* 2"/.test(kept)) fail("the formula built with the strip was not saved as typed");

// In the form that adds an element
await page.click('[data-add="year"]');
const slot = page.locator('[data-addslot="year"]');
await slot.locator('[data-akind="formula"]').click();
const af = slot.locator("[data-aform]");
await af.fill("=MA");
await af.evaluate((el) => el.setSelectionRange(el.value.length, el.value.length));
await af.dispatchEvent("input");
await slot.locator("[data-sug]", { hasText: /^MAX\($/ }).waitFor({ timeout: 30000 }).catch(() => {});
const addFns = await slot.locator("[data-sug]").allTextContents();
if (!addFns.includes("MAX(")) fail(`in the Add form, typing "MA" offers ${addFns.join(", ") || "nothing"}, not MAX(`);

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.slice(0, 40).join("\n") + (failures.length > 40 ? `\n… and ${failures.length - 40} more` : ""));
  process.exit(1);
}
console.log(`Suggestions: ${result.offered} of ${result.words} names and functions in ${result.formulas} formulas are offered by their first three letters and complete to the same word; only Forge's ${result.total} functions are offered; the strip works on a card and in the Add form.`);
