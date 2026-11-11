// Open the published Forge Studio and check what the roadmap's S4 asks: the
// page runs the release's version of Forge, and the bagel example's
// year.profit is 786.32.
//
//   node studio/tests/live.mjs <address> <version>

import { chromium } from "playwright";

const [address, version] = process.argv.slice(2);
if (!address || !version) {
  console.error("usage: node studio/tests/live.mjs <address> <version>");
  process.exit(2);
}

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
const errors = [];
page.on("pageerror", (e) => errors.push(e.message));
page.on("console", (m) => { if (m.type() === "error") errors.push(m.text()); });

// A new site can take a minute to be served
let loaded = false;
for (let attempt = 1; attempt <= 12 && !loaded; attempt++) {
  const response = await page.goto(`${address.replace(/\/$/, "")}/?example=bagel`).catch(() => null);
  loaded = !!response && response.ok();
  if (!loaded) await new Promise((r) => setTimeout(r, 10000));
}
if (!loaded) { console.error(`${address} did not answer`); process.exit(1); }

await page.waitForFunction(() => document.body.dataset.state !== "loading", null, { timeout: 120000 });
const state = await page.evaluate(() => document.body.dataset.state);
const status = (await page.textContent("#status-text")).trim();
const profit = Number(await page.getAttribute("[data-scalar='year.profit']", "data-value"));
await browser.close();

const failures = [];
if (state !== "ready") failures.push(`the page says ${state}`);
if (status !== `Forge ${version}`) failures.push(`the page shows "${status}", the release is ${version}`);
if (Math.abs(profit - 786.32) > 0.005) failures.push(`year.profit is ${profit}, not 786.32`);
for (const e of errors) failures.push(`browser error: ${e}`);
if (failures.length) { console.error(failures.join("\n")); process.exit(1); }
console.log(`${address}: Forge ${version}, year.profit ${profit.toFixed(2)}`);
