// The screenshots of Forge Studio in the main README, taken from the page
// itself so that they can be made again after a release:
//
//   node studio/screenshots.mjs <built Studio dir or address> <out dir>
//
// studio-phone.png is an iPhone-sized view of the bagel example's results;
// studio-desktop.png the editor and the cards side by side.

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
const ready = (page) => page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });

const browser = await chromium.launch();

// Phone: the year's results, under the scenarios
{
  const page = await browser.newPage({ viewport: { width: 390, height: 844 }, deviceScaleFactor: 2, isMobile: true, hasTouch: true, colorScheme: "light" });
  await page.goto(`${base}/?example=bagel`);
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
  await page.goto(`${base}/?example=bagel`);
  await ready(page);
  await page.waitForTimeout(400);
  await page.screenshot({ path: join(out, "studio-desktop.png") });
  await page.close();
}
await browser.close();
if (!remote) server.close();
console.log(`Screenshots in ${out}`);
