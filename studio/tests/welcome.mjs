// The welcome card on a first visit (ADR-056 addendum), in a headless browser:
//
// - a first visit shows it above the model, with its three tips and Got it;
// - Got it removes it, and it stays away after a reload (a flag on the
//   device, in localStorage);
// - "Show the welcome tips again" on the Files tab brings it back;
// - the card leaves the model alone: changing an input with the card on
//   screen still recalculates;
// - at 320 px wide the card does not make the page scroll sideways.
//
//   node studio/tests/welcome.mjs <built Studio dir or address>

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
const ready = (page) => page.waitForFunction(() => document.body.dataset.state !== "loading" && document.querySelector("[data-scalar]"), null, { timeout: 120000 });
const generation = (page) => page.evaluate(() => Number(document.body.dataset.generation));
const settle = (page, before) => page.waitForFunction((g) => Number(document.body.dataset.generation) > g && document.body.dataset.state !== "loading", before, { timeout: 60000 });

const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
const page = await context.newPage();
page.on("pageerror", (e) => fail(`page error: ${e.message}`));
await page.goto(`${base}/`);
await ready(page);

// A first visit
const card = page.locator("#welcome");
if (!(await card.isVisible())) fail("a first visit does not show the welcome card");
else {
  const tips = await card.locator("li").count();
  if (tips !== 3) fail(`the welcome card has ${tips} tips, not 3`);
  if (!/leaves this device/.test(await card.innerText())) fail("the welcome card does not say that nothing leaves the device");
}

// The card leaves the model alone
let before = await generation(page);
await page.fill('[data-input="assumptions.open_days_per_month"]', "28");
await settle(page, before);
if ((await page.getAttribute('[data-scalar="year.profit"]', "data-value")) === "786.320362") fail("with the welcome card on screen, changing an input did not recalculate");

// Got it, then a reload
await page.click("#welcome-done");
if (await card.count()) fail("Got it did not remove the welcome card");
await page.reload();
await ready(page);
if (await page.locator("#welcome").count()) fail("after Got it and a reload, the welcome card came back");

// Back from the Files tab
await page.click('.tab[data-tab="files"]');
await page.click("#welcome-again");
if (!(await page.locator("#welcome").isVisible())) fail("Show the welcome tips again did not bring the card back on the Model tab");

// Narrow screens
const narrow = await browser.newPage({ viewport: { width: 320, height: 700 } });
await narrow.goto(`${base}/`);
await ready(narrow);
const wide = await narrow.evaluate(() => document.documentElement.scrollWidth);
if (wide > 320) fail(`at 320 px the page with the welcome card is ${wide} px wide`);

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log("Welcome: a first visit shows the card with three tips, the model still recalculates under it, Got it keeps it away after a reload, Files brings it back, and it fits 320 px.");
