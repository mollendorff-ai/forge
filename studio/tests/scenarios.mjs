// Forge Studio's scenarios beside the weighted ones (ADR-067), in a headless
// browser, on the acquisition example:
//
// - deal.yaml: the Analyze table has the four weighted scenarios Forge
//   returns, the note below it names all_cash and all_shares, and their chips
//   show no percentage while base shows 45%;
// - deal_risk.yaml, whose one scenario has no probability: the Analyze card
//   gives Forge's reason, not "This model has no scenarios section", and
//   says what it means (choices to compare, not chances), with no
//   "Validation error" (ADR-076).
//
//   node studio/tests/scenarios.mjs <built Studio dir or address>

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

const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
page.on("pageerror", (e) => fail(`acquisition: ${e.message}`));
await page.goto(`${base}/?example=acquisition`);
await ready(page);

// The chips: a percentage for a chance, none for a choice
const chip = (name) => page.textContent(`[data-scn="${name}"]`);
if (!/45%/.test(await chip("base"))) fail(`the base chip shows no 45%: ${await chip("base")}`);
for (const name of ["all_cash", "all_shares"]) {
  if (await page.locator(`[data-scn="${name}"] small`).count()) fail(`the ${name} chip shows a percentage`);
}

// Analyze: the weighted scenarios are Forge's, and the choices are named
const want = await page.evaluate(async () => {
  const { call } = await import("./forge.js");
  const set = (await (await fetch("examples.json")).json()).find((x) => x.id === "acquisition");
  const r = await call("scenarios", { files: set.files, main: set.main });
  return { weighted: r.scenarios.map((s) => s.name), unweighted: r.unweighted };
});
if (want.weighted.length !== 4) fail(`Forge weighs ${want.weighted.length} scenarios, not 4`);
if (want.unweighted.join(",") !== "all_cash,all_shares") fail(`Forge leaves out ${want.unweighted}, not all_cash and all_shares`);
await page.click('.tab[data-tab="analyze"]');
await page.waitForSelector("#scn-box table");
const rows = await page.$$eval("#scn-box tbody th", (ths) => ths.map((th) => th.textContent));
if (rows.join(",") !== want.weighted.join(",")) fail(`the Analyze table has ${rows}, not Forge's ${want.weighted}`);
const note = (await page.textContent("#scn-unweighted").catch(() => "")) || "";
if (!note.includes("all_cash, all_shares")) fail(`the Analyze card does not name the scenarios left out: "${note}"`);

// deal_risk.yaml: no scenario to weigh, and Studio says why
await page.click('.tab[data-tab="files"]');
await page.click('[data-main="deal_risk.yaml"]');
await ready(page);
await page.click('.tab[data-tab="analyze"]');
await page.waitForSelector("#scn-box");
const reason = await page.textContent("#scn-box");
if (!reason.includes("No scenario has a probability")) fail(`deal_risk.yaml: the Analyze card does not give Forge's reason: "${reason}"`);
if (reason.includes("no scenarios section")) fail("deal_risk.yaml: the Analyze card says the model has no scenarios");
// What Forge's message means, in the page's words, and not its error kind (ADR-076)
if (!reason.includes("choices to compare, not chances")) fail(`deal_risk.yaml: the Analyze card does not say what Forge's message means: "${reason}"`);
if (/Validation error/.test(reason)) fail(`deal_risk.yaml: the Analyze card shows "Validation error": "${reason}"`);
await page.close();

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log("Scenarios: Studio weighs the four scenarios Forge weighs, names the two left out, shows no chance on a choice, and gives Forge's reason when there is nothing to weigh.");
