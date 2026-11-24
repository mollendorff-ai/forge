// The screenshots of Forge Studio in the main README, taken from the page
// itself so that they can be made again after a release:
//
//   node studio/screenshots.mjs <built Studio dir or address> <out dir>
//
// studio-phone.png is an iPhone-sized view of the results of the bagel example's uncertain model (bagel_simulate.yaml: the plan, and the range under each output);
// studio-desktop.png the editor and the cards side by side.
//
// With a third argument, the screenshots of the Studio guide (docs/guide/studio.md)
// too, each named in that page, in that folder:
//
//   node studio/screenshots.mjs <built Studio dir or address> <out dir> docs/guide/img

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync, mkdirSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";

const [where, outArg, guideArg] = process.argv.slice(2);
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
if (guideArg) {
  const guide = resolve(guideArg);
  mkdirSync(guide, { recursive: true });
  const phone = { viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: "light" };
  const desktop = { viewport: { width: 1200, height: 800 }, deviceScaleFactor: 1, colorScheme: "light" };
  const open = async (options, query) => {
    const page = await browser.newPage(options);
    await page.goto(`${base}/${query}`);
    await page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });
    return page;
  };
  const scrollTo = (page, heading) => page.evaluate((name) => {
    const h = [...document.querySelectorAll("#view-model .eyebrow h2")].find((e) => e.textContent.trim() === name);
    const top = document.querySelector(".top").getBoundingClientRect().height;
    scrollTo(0, h.getBoundingClientRect().top + scrollY - top - 12);
  }, heading);
  // A first visit: the welcome card and the questions it offers
  {
    const page = await open(phone, "?example=bagel");
    await page.waitForTimeout(400);
    await page.screenshot({ path: join(guide, "studio-welcome.png") });
    await page.close();
  }
  // The price raised by a dollar: the year's results move
  {
    const page = await open(desktop, "?example=bagel");
    await page.locator("#welcome button", { hasText: "Got it" }).click();
    await page.fill("#in-assumptions_price_per_dozen", "17");
    await page.press("#in-assumptions_price_per_dozen", "Enter");
    await page.waitForFunction(() => document.body.dataset.state === "ready");
    await scrollTo(page, "assumptions");
    await page.waitForTimeout(600);
    await page.screenshot({ path: join(guide, "studio-price.png") });
    await page.close();
  }
  // The grid: the model as a spreadsheet reads it
  {
    const page = await open(desktop, "?example=bagel");
    await page.locator("#welcome button", { hasText: "Got it" }).click();
    await page.locator(".tab[data-tab=grid]").click();
    await page.waitForTimeout(400);
    await page.screenshot({ path: join(guide, "studio-grid.png") });
    await page.close();
  }
  // Analyze: what the model can be asked
  {
    const page = await open(desktop, "?example=bagel");
    await page.locator("#welcome button", { hasText: "Got it" }).click();
    await page.locator(".tab[data-tab=analyze]").click();
    await page.waitForTimeout(600);
    await page.screenshot({ path: join(guide, "studio-analyze.png") });
    await page.close();
  }
  // The ? on a heading
  {
    const page = await open(phone, "?example=bagel");
    await page.locator("#welcome button", { hasText: "Got it" }).click();
    await scrollTo(page, "assumptions");
    await page.locator("#view-model .eyebrow", { hasText: "assumptions" }).locator("button", { hasText: "?" }).click();
    await page.waitForTimeout(400);
    await page.screenshot({ path: join(guide, "studio-help.png") });
    await page.close();
  }
  // The year's results, after the same change, on a phone
  {
    const page = await open(phone, "?example=bagel");
    await page.locator("#welcome button", { hasText: "Got it" }).click();
    await page.fill("#in-assumptions_price_per_dozen", "17");
    await page.press("#in-assumptions_price_per_dozen", "Enter");
    await page.waitForFunction(() => document.body.dataset.state === "ready");
    await scrollTo(page, "year");
    await page.waitForTimeout(600);
    await page.screenshot({ path: join(guide, "studio-year.png") });
    await page.close();
  }
  console.log(`Guide screenshots in ${guide}`);
}
await browser.close();
if (!remote) server.close();
console.log(`Screenshots in ${out}`);
