// A `format:` on screen (ADR-061), in a headless browser with Forge's own
// text: Studio shows what Forge gives for a number that has a format, on the
// cards and in the grid, and adds no rule of its own. The expected text is
// what `forge calculate` returns in `formatted` for the bagel and the rolling
// forecast (both carry formats).
//
//   node studio/tests/formats.mjs <built Studio dir or address>

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
const ready = (page) => page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });
const text = async (page, selector) => ((await page.textContent(selector, { timeout: 15000 })) ?? "").trim();
const expect = async (page, label, selector, want) => {
  const got = await text(page, selector).catch((e) => `(${e.message.split("\n")[0]})`);
  if (got !== want) fail(`${label}: Studio shows ${JSON.stringify(got)}, Forge gives ${JSON.stringify(want)}`);
  return got === want ? 1 : 0;
};

let checked = 0;
const open = async (id) => {
  const page = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  page.on("pageerror", (e) => fail(`${id}: ${e.message}`));
  await page.goto(`${base}/?example=${id}`);
  await ready(page);
  return page;
};

// The bagel: a scalar, a data column and two formula columns carry a format
{
  const page = await open("bagel");
  checked += await expect(page, "bagel price hint", '[data-card="assumptions.price_per_dozen"] [data-hint]', "$16.00");
  checked += await expect(page, "bagel margin card", '[data-card="year.margin"] [data-val]', "0.2%");
  await page.click('.tab[data-tab="grid"]');
  checked += await expect(page, "bagel season, row 1", '#view-grid [data-cell="months.season"][data-row="0"]', "85%");
  checked += await expect(page, "bagel season, row 2", '#view-grid [data-cell="months.season"][data-row="1"]', "125%");
  checked += await expect(page, "bagel dozens, row 1", '#view-grid [data-cell="months.dozens"][data-row="0"]', "1,403");
  checked += await expect(page, "bagel margin in the list", '#view-grid [data-gscalar="year.margin"] td', "0.2%");
  // The value underneath is Forge's number, not the text
  const raw = await page.getAttribute('#view-grid [data-cell="months.season"][data-row="0"]', "data-value");
  if (raw !== "0.85") fail(`bagel season: the grid holds ${raw}, not 0.85`);
  await page.close();
}

// The rolling forecast: a date, percentages, counts and a negative-in-brackets format
{
  const page = await open("forecast");
  checked += await expect(page, "forecast last_actual hint", '[data-card="inputs.last_actual"] [data-hint]', "31 Oct 2026");
  checked += await expect(page, "forecast growth hint", '[data-card="inputs.standard_volume_growth"] [data-hint]', "0.8%");
  checked += await expect(page, "forecast fy2027 revenue", '[data-card="fy2027.revenue"] [data-val]', "987,622");
  checked += await expect(page, "forecast fy2027 variance", '[data-card="fy2027.variance"] [data-val]', "27,022");
  await page.click('.tab[data-tab="grid"]');
  checked += await expect(page, "forecast units, row 1", '#view-grid [data-cell="months.standard_units"][data-row="0"]', "985");
  checked += await expect(page, "forecast price, row 1", '#view-grid [data-cell="months.standard_price"][data-row="0"]', "50.00");
  checked += await expect(page, "forecast revenue, row 2", '#view-grid [data-cell="months.revenue"][data-row="1"]', "77,560");
  checked += await expect(page, "forecast share, quarter 1", '#view-grid [data-cell="quarters.share_of_year"][data-row="0"]', "23.2%");
  checked += await expect(page, "forecast variance, quarter 2", '#view-grid [data-cell="quarters.variance"][data-row="1"]', "5,679");
  await page.close();
}

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`Formats: ${failures.length} problems\n` + failures.map((f) => "  - " + f).join("\n"));
  process.exit(1);
}
console.log(`Formats: ${checked} numbers on the cards and in the grid of the bagel and the rolling forecast are the text Forge gives, with the values underneath unchanged.`);
