// What a model states its results should be (expect:, ADR-074), in a
// headless browser, on the stag hunt of the Games set:
//
// - in the plan and under a scenario, a result with an expectation shows it
//   on its card, as Forge's calculate reports it, marked when they differ;
// - an input typed on a card that moves a result away from its expectation
//   is kept (Studio's check of an edit leaves the expectations out), and the
//   card says what expect: states and what Forge gives.
//
//   node studio/tests/expect.mjs <built Studio dir or address>

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
const ready = (page) => page.waitForFunction(() => document.body.dataset.state !== "loading", null, { timeout: 120000 });
const generation = (page) => page.evaluate(() => Number(document.body.dataset.generation));
const settle = (page, before) => page.waitForFunction((g) => Number(document.body.dataset.generation) > g && document.body.dataset.state !== "loading", before, { timeout: 60000 });

const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
page.on("pageerror", (e) => fail(`stag hunt: ${e.message}`));
await page.goto(`${base}/?example=games`);
await ready(page);

// Forge's own report of a run's expectations, from the files on screen
const forge = (scenario) => page.evaluate(async (scenario) => {
  const { call } = await import("./forge.js");
  const set = (await (await fetch("examples.json")).json()).find((x) => x.id === "games");
  const files = JSON.parse(localStorage.getItem("forge-studio-v3") || "null")?.files || set.files;
  const r = await call("calculate", { files, main: "stag-hunt.yaml", scenario });
  return r.expectations || [];
}, scenario);
const card = (path) => page.evaluate((path) => {
  const c = document.querySelector(`#view-model [data-card="${path}"]`);
  const e = c && c.querySelector("[data-expect]");
  return c && { state: c.dataset.expected || "", text: e && !e.hidden ? e.textContent : "", value: c.dataset.value };
}, path);

// Each card with an expectation shows Forge's check of it
async function compare(label, scenario) {
  const checks = await forge(scenario);
  if (!checks.length) fail(`${label}: Forge reports no expectation`);
  for (const check of checks) {
    const shown = await card(check.path);
    if (!shown) { fail(`${label}: no card for ${check.path}`); continue; }
    const want = check.met ? "met" : "differs";
    if (shown.state !== want) fail(`${label}: ${check.path} is marked "${shown.state}", Forge says ${want}`);
    if (check.formula && !shown.text.includes(`(${check.formula})`)) fail(`${label}: ${check.path} does not show ${check.formula}: "${shown.text}"`);
    if (shown.value !== String(check.got)) fail(`${label}: ${check.path} shows ${shown.value}, Forge gives ${check.got}`);
  }
  return checks;
}

// The plan: four results stated by hand, all met
const plan = await compare("plan", null);
if (plan.length !== 4 || !plan.every((c) => c.met)) fail(`plan: Forge's checks are ${JSON.stringify(plan)}`);
if (!(await card("kinship.stag_share")).text.includes("(=3/4)")) fail("plan: the mixed equilibrium's card does not show =3/4");

// A scenario: its own expectations
let before = await generation(page);
await page.click('[data-scn="c3"]');
await settle(page, before);
const c3 = await compare("c3", "c3");
if (c3.map((c) => c.formula || c.expected).join(",") !== "=3/8,1") fail(`c3: Forge's checks are ${JSON.stringify(c3)}`);
if ((await card("kinship.stags_at_the_end")).text) fail("c3: a result with no expectation under c3 shows one");

// An edit that moves a result away from its expectation is kept, and says so
before = await generation(page);
await page.click('[data-scn=""]');
await settle(page, before);
before = await generation(page);
await page.fill('[data-input="c"]', "0.2");
await settle(page, before);
const moved = await compare("c = 0.2", null);
const share = moved.find((c) => c.path === "kinship.stag_share");
if (!share || share.met || share.got !== 0.7) fail(`c = 0.2: Forge's check of the mix is ${JSON.stringify(share)}, not 0.7 against 3/4`);
const text = (await card("kinship.stag_share")).text;
if (!/^expect: states .*\(=3\/4\), and Forge gives /.test(text)) fail(`c = 0.2: the card says "${text}"`);
const kept = await page.evaluate(() => JSON.parse(localStorage.getItem("forge-studio-v3") || "null")?.files?.["stag-hunt.yaml"] || "");
if (!/^c: 0\.2\b/m.test(kept)) fail("c = 0.2: the edit was not kept in the file");
await page.close();

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log("Studio shows each expectation as Forge checks it, in the plan, under a scenario and after an edit");
