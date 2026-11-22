// An uncertain model on Forge Studio's Model view (ADR-076), in a headless
// browser at 390 px, for every uncertain model of the example sets:
//
// - every scalar on screen is `forge calculate --plan`'s, natively, and the
//   pill says "uncertain: N inputs are ranges", N being Forge's, not Problem;
// - each output of `monte_carlo.outputs` has its band (a column, a table of
//   its rows), and every number of it (P5, the mean, P95, each threshold) is
//   `forge simulate`'s with the model's own runs, seed and sampling, natively,
//   at the place in the answer it names; so is each distribution's mean;
// - each input that is a distribution is a card with Forge's kinds, and its
//   parameters named as Forge's function list names them;
// - nothing is wider than 390 px.
//
// On the bagel's simulation, also: Analyze lists the outputs in the model's
// order; a typed input dims the band ("running") and the new band is
// simulate's for the new files, and of two changes in a row only the last
// one's is kept; a parameter typed on a distribution card and another kind
// chosen write the formula, which Forge reads back.
//
//   node studio/tests/uncertain.mjs <built Studio dir> <expected.json>

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";
import { SETS } from "../build.mjs";

const [where, expectedPath] = process.argv.slice(2);
const dir = resolve(where);
const expected = JSON.parse(readFileSync(resolve(expectedPath), "utf8"));
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".wasm": "application/wasm" };
const server = createServer((req, res) => {
  let file = join(dir, decodeURIComponent(new URL(req.url, "http://x").pathname));
  if (existsSync(file) && statSync(file).isDirectory()) file = join(file, "index.html");
  if (!file.startsWith(dir) || !existsSync(file)) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { "content-type": TYPES[extname(file)] || "application/octet-stream" });
  res.end(readFileSync(file));
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const base = `http://127.0.0.1:${server.address().port}`;

const failures = [];
const fail = (m) => failures.push(m);
const close = (a, b) => Math.abs(a - b) <= 1e-9 * Math.max(1, Math.abs(a), Math.abs(b));
const at = (value, pointer) => pointer.split("/").slice(1).map((p) => p.replace(/~1/g, "/").replace(/~0/g, "~")).reduce((v, k) => (v == null ? undefined : v[k]), value);
const banded = (page) => page.waitForFunction(() => document.body.dataset.state !== "loading" && document.body.dataset.band === "ready", null, { timeout: 180000 });

// Every uncertain model of the sets has a native answer, and no other
const natives = Object.keys(expected).filter((k) => k.startsWith("uncertain: "));
const models = SETS.flatMap((set) => set.files
  .filter((f) => /^monte_carlo:/m.test(readFileSync(new URL(`../../examples/${set.dir}/${f}`, import.meta.url), "utf8")))
  .map((f) => ({ set, file: f, key: `uncertain: ${set.dir}/${f}` })));
for (const m of models) if (!natives.includes(m.key)) fail(`${m.key}: no native answer in expected.json`);
if (models.length < 7) fail(`only ${models.length} uncertain models in the sets`);

const browser = await chromium.launch();
const functions = [];

for (const { set, file, key } of models) {
  const native = expected[key];
  if (!native) continue;
  const plan = native.calculate, sim = native.simulate, label = `${set.id}/${file}`;
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  page.on("pageerror", (e) => fail(`${label}: ${e.message}`));
  await page.goto(`${base}/?example=${set.id}&model=${file}`);
  await banded(page);

  // The plan on every card
  const shown = new Map(await page.$$eval("[data-scalar]", (els) => els.map((e) => [e.dataset.scalar, e.dataset.value])));
  for (const [path, value] of Object.entries(plan.scalars)) {
    if (!shown.has(path)) { fail(`${label}: ${path} is not on screen`); continue; }
    if (value === null ? shown.get(path) !== "" : !close(Number(shown.get(path)), value)) fail(`${label}: ${path} shows ${shown.get(path)}, forge calculate --plan gives ${value}`);
  }
  for (const path of shown.keys()) if (!(path in plan.scalars)) fail(`${label}: ${path} is on screen but not in forge calculate --plan's answer`);

  // The pill and the note: uncertain, with Forge's count, not Problem
  const n = Object.keys(plan.at_plan).length;
  const pill = await page.textContent("#status-text");
  const want = `uncertain: ${n} input${n === 1 ? " is a range" : "s are ranges"}`;
  if (pill !== want) fail(`${label}: the pill says "${pill}", not "${want}"`);
  if (await page.isVisible("#model-error")) fail(`${label}: a problem is shown: ${await page.textContent("#model-error")}`);
  if (!(await page.textContent("#model-uncertain")).includes(want)) fail(`${label}: the note does not say "${want}"`);

  // Every number of every band, and each distribution's mean, is simulate's
  const nums = await page.$$eval("#view-model [data-num]", (els) => els.map((e) => [e.dataset.num, e.dataset.value]));
  for (const [pointer, value] of nums) {
    const w = at(sim, pointer);
    if (typeof w !== "number" || !close(Number(value), w)) fail(`${label}: ${pointer} shows ${value}, forge simulate gives ${w}`);
  }
  const pointers = new Set(nums.map(([p]) => p));
  const r = sim.monte_carlo_results;
  const enc = (s) => s.replace(/~/g, "~0").replace(/\//g, "~1");
  for (const name of r.output_order) {
    const o = r.outputs[name], out = `/monte_carlo_results/outputs/${enc(name)}`;
    for (const p of [`${out}/percentiles/p5`, `${out}/mean`, `${out}/percentiles/p95`, ...Object.keys(o.thresholds).map((t) => `${out}/thresholds/${enc(t)}`)]) {
      if (!pointers.has(p)) fail(`${label}: ${p} is not on screen`);
    }
  }
  if (r.output_order.length !== Object.keys(r.outputs).length) fail(`${label}: output_order has ${r.output_order.length} of ${Object.keys(r.outputs).length} outputs`);

  // A distribution is a card, with Forge's kinds and parameter names
  if (!functions.length) {
    functions.push(...await page.evaluate(async () => {
      const { call } = await import("./forge.js");
      return (await call("functions", {})).categories.flatMap((c) => c.functions).filter((f) => f.name.startsWith("MC."));
    }));
  }
  const kinds = functions.map((f) => f.name);
  const scalarInputs = Object.keys(plan.at_plan).filter((k) => !/ \(row \d+\)$/.test(k));
  for (const path of scalarInputs) {
    const card = page.locator(`[data-dist="${path}"]`);
    if (!(await card.count())) { fail(`${label}: ${path} has no distribution card`); continue; }
    const chips = await card.locator("[data-dkind]").evaluateAll((els) => els.map((e) => e.dataset.dkind));
    if (chips.join() !== kinds.join()) fail(`${label}: ${path}'s kinds are ${chips}, not Forge's ${kinds}`);
    const kind = await card.locator('[data-dkind][aria-pressed="true"]').getAttribute("data-dkind");
    const f = functions.find((x) => x.name === kind);
    const params = /\(([^)]*)\)/.exec(f.syntax)[1].split(",").map((p) => p.trim());
    const labels = await card.locator(".dist-params label").allTextContents();
    if (labels.join() !== params.join()) fail(`${label}: ${path} has fields ${labels}, Forge's ${f.syntax} names ${params}`);
    const mean = `/monte_carlo_results/inputs/${enc(path)}/mean`;
    if (!pointers.has(mean)) fail(`${label}: ${path} does not show its mean (${mean})`);
  }

  const wide = await page.evaluate(() => document.documentElement.scrollWidth);
  if (wide > 390) fail(`${label}: the page is ${wide} px wide at 390 px`);
  if (set.id === "bagel") await bagel(page, r);
  await page.close();
}

// The bagel's simulation, changed on its cards
async function bagel(page, r) {
  const simulateNow = () => page.evaluate(async () => {
    const { call } = await import("./forge.js");
    const s = JSON.parse(localStorage.getItem("forge-studio-v3"));
    return call("simulate", { files: s.files, main: s.main });
  });
  const bandNumbers = () => page.$$eval("#view-model [data-num]", (els) => els.map((e) => [e.dataset.num, Number(e.dataset.value)]));
  const sameAs = async (answer, what) => {
    for (const [pointer, value] of await bandNumbers()) {
      if (!close(value, at(answer, pointer))) { fail(`bagel: ${what}: ${pointer} shows ${value}, simulate gives ${at(answer, pointer)}`); return; }
    }
  };

  // A typed input: the band dims while it runs, then is simulate's again
  const field = page.locator('[data-input="assumptions.price_per_dozen"]');
  await field.fill("17");
  await field.dispatchEvent("input");
  await page.waitForFunction(() => document.body.dataset.band === "running", null, { timeout: 5000 }).catch(() => fail("bagel: the band is not marked running after a change"));
  const dim = await page.locator(".band").first().evaluate((e) => getComputedStyle(e).opacity);
  if (!(Number(dim) < 1)) fail(`bagel: the band is not dimmed while it runs (opacity ${dim})`);
  await banded(page);
  await sameAs(await simulateNow(), "price 17");
  const moved = (await bandNumbers()).find(([p]) => p === "/monte_carlo_results/outputs/year.profit/mean");
  if (!moved || close(moved[1], r.outputs["year.profit"].mean)) fail("bagel: the year's mean did not move with the price");

  // Two changes in a row: only the last one's band is kept
  await field.fill("18");
  await field.dispatchEvent("input");
  await page.waitForTimeout(150);
  await field.fill("19");
  await field.dispatchEvent("input");
  await banded(page);
  await page.waitForTimeout(1500);
  await sameAs(await simulateNow(), "price 19 after 18");

  // A distribution card: a parameter typed, then another kind
  const card = page.locator('[data-dist="assumptions.opening_dozens_per_day"]');
  const before = Number(await page.evaluate(() => document.body.dataset.generation));
  await card.locator('[data-dparam="2"]').fill("75");
  await card.locator('[data-dparam="2"]').dispatchEvent("change");
  await page.waitForFunction((g) => Number(document.body.dataset.generation) > g, before, { timeout: 30000 });
  await banded(page);
  let formula = await page.locator('[data-dist="assumptions.opening_dozens_per_day"] .formula').textContent();
  if (formula !== "=MC.PERT(40, 55, 75)") fail(`bagel: the typed maximum gives ${formula}, not =MC.PERT(40, 55, 75)`);
  const text = await page.evaluate(() => JSON.parse(localStorage.getItem("forge-studio-v3")).files["bagel_simulate.yaml"]);
  if (!text.includes('formula: "=MC.PERT(40, 55, 75)"')) fail("bagel: the file does not hold the new formula");
  await sameAs(await simulateNow(), "a wider demand");
  const again = Number(await page.evaluate(() => document.body.dataset.generation));
  await page.click('[data-dist="assumptions.opening_dozens_per_day"] [data-dkind="MC.Triangular"]');
  await page.waitForFunction((g) => Number(document.body.dataset.generation) > g, again, { timeout: 30000 });
  await banded(page);
  formula = await page.locator('[data-dist="assumptions.opening_dozens_per_day"] .formula').textContent();
  if (formula !== "=MC.Triangular(40, 55, 75)") fail(`bagel: Triangular gives ${formula}, not =MC.Triangular(40, 55, 75), its min, mode and max kept`);

  // Analyze: the outputs in the model's order
  await page.click('.tab[data-tab="analyze"]');
  await page.click("#sim-run");
  await page.waitForSelector("#sim-out", { timeout: 60000 });
  const options = await page.$$eval("#sim-out option", (os) => os.map((o) => o.value));
  if (options.join("|") !== r.output_order.join("|")) fail(`bagel: Analyze lists ${options.slice(0, 5)}…, not the model's order ${r.output_order.slice(0, 5)}…`);
}

await browser.close();
server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.slice(0, 60).join("\n"));
  process.exit(1);
}
console.log(`Uncertain models: ${models.length} models at plan as forge calculate --plan gives them, every band and mean forge simulate's, distribution cards from Forge's function list; the bagel's band follows a change and its distributions are written.`);
