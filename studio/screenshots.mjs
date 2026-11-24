// The screenshots of Forge Studio in the main README, taken from the page
// itself so that they can be made again after a release:
//
//   node studio/screenshots.mjs <built Studio dir or address> <out dir>
//
// studio-phone.png is an iPhone-sized view of the results of the bagel example's uncertain model (bagel_simulate.yaml: the plan, and the range under each output);
// studio-desktop.png the editor and the cards side by side; studio-guide-*.png
// are the guide's walk through (docs/guide/studio.md). Every file name is written
// out in full, so that a test can check that each image of docs/images is made here.

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync, mkdirSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";

const [where, outArg] = process.argv.slice(2);
const out = resolve(outArg || "docs/images");
mkdirSync(out, { recursive: true });
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
// Ready when the model is calculated, its ranges are worked out (an uncertain
// model has them) and the welcome card is put away
const ready = async (page) => {
  await page.waitForFunction(() => document.body.dataset.state === "ready" && ["ready", "none"].includes(document.body.dataset.band), null, { timeout: 120000 });
  const gotIt = page.locator("#welcome button", { hasText: "Got it" });
  if (await gotIt.isVisible()) await gotIt.click();
};

const browser = await chromium.launch();

// Phone: the year's results, under the scenarios
{
  const page = await browser.newPage({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: "light" });
  await page.goto(`${base}/?example=bagel&model=bagel_simulate.yaml`);
  await ready(page);
  await page.evaluate(() => {
    const h = [...document.querySelectorAll("#view-model .eyebrow h2")].find((e) => e.textContent.trim() === "year");
    const top = document.querySelector(".top").getBoundingClientRect().height;
    scrollTo(0, h.getBoundingClientRect().top + scrollY - top - 12);
  });
  await page.waitForTimeout(400);
  await page.screenshot({ path: join(out, "studio-phone.png") });
  await page.close();
}
// Desktop: the editor and the cards
{
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 }, deviceScaleFactor: 1, colorScheme: "light" });
  await page.goto(`${base}/?example=bagel&model=bagel_simulate.yaml`);
  await ready(page);
  await page.waitForTimeout(400);
  await page.screenshot({ path: join(out, "studio-desktop.png") });
  await page.close();
}
// The guide's walk through Studio (docs/guide/studio.md): the bagel's plan on a
// phone, one view after another, each as a person meets it
{
  const page = await browser.newPage({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: "light" });
  await page.goto(`${base}/?example=bagel`);
  await ready(page);
  const shot = async (file) => { await page.waitForTimeout(400); await page.screenshot({ path: join(out, file) }); };
  const scrollTo = (heading) => page.evaluate((text) => {
    const h = [...document.querySelectorAll("#view-model .eyebrow h2")].find((e) => e.textContent.trim() === text);
    const top = document.querySelector(".top").getBoundingClientRect().height;
    scrollTo(0, h.getBoundingClientRect().top + scrollY - top - 12);
  }, heading);
  // The model: the strip of questions, then a group of inputs
  await shot("studio-guide-model.png");
  // An input changed: the price, typed, and the year's results under it
  await scrollTo("assumptions");
  const price = page.locator('[data-input="assumptions.price_per_dozen"]');
  const typed = async (value) => {
    const before = Number(await page.evaluate(() => document.body.dataset.generation));
    await price.fill(value);
    await page.waitForFunction((g) => Number(document.body.dataset.generation) > g && document.body.dataset.state === "ready", before, { timeout: 60000 });
  };
  await typed("14");
  await shot("studio-guide-input.png");
  // A result's formula
  await scrollTo("year");
  await page.locator('[data-edit="year.profit"]').click();
  await shot("studio-guide-formula.png");
  await scrollTo("assumptions");
  await typed("16"); // back to the plan for the other views
  // The other views
  for (const tab of ["grid", "analyze", "mcp", "files"]) {
    await page.click(`[data-tab="${tab}"]`);
    if (tab === "analyze") { await page.click("#tor-run"); await page.waitForSelector("#tor-box .tor-row", { timeout: 60000 }); await page.locator("#tor-box").scrollIntoViewIfNeeded(); }
    if (tab === "files") await page.evaluate(() => {
      const h = [...document.querySelectorAll("#view-files .eyebrow h2")].find((e) => e.textContent.trim() === "save");
      scrollTo(0, h.getBoundingClientRect().top + scrollY - document.querySelector(".top").getBoundingClientRect().height - 12);
    });
    await shot({ grid: "studio-guide-grid.png", analyze: "studio-guide-analyze.png", mcp: "studio-guide-tools.png", files: "studio-guide-files.png" }[tab]);
  }
  await page.click('[data-tab="model"]');
  await page.locator("[data-help]").first().click();
  await shot("studio-guide-help.png");
  await page.close();
}
await browser.close();
if (!remote) server.close();
console.log(`Screenshots in ${out}`);
