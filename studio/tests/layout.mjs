// Forge Studio's layouts (roadmap S5, ADR-056 decision 3), in a headless
// browser:
//
// - at 320 px wide, no view scrolls sideways;
// - at 1440 x 900 the editor and the cards are side by side, typing a number
//   in the editor changes its card, a change on a card changes the editor,
//   the divider moves and either side can be hidden;
// - with a folder as third argument, screenshots at 390 x 844 and 1440 x 900,
//   light and dark, for the pull request.
//
//   node studio/tests/layout.mjs <built Studio dir or address> [screenshots dir]

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync, mkdirSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";

const [where, shots] = process.argv.slice(2);
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
const settle = async (page, before) => {
  await page.waitForFunction((g) => Number(document.body.dataset.generation) > g && document.body.dataset.state !== "loading", before, { timeout: 60000 });
};
const generation = (page) => page.evaluate(() => Number(document.body.dataset.generation));
const value = (page, path) => page.getAttribute(`[data-scalar="${path}"]`, "data-value");

async function open(viewport, example, colorScheme = "light") {
  const page = await browser.newPage({ viewport, colorScheme });
  page.on("pageerror", (e) => fail(`${example} at ${viewport.width} px: ${e.message}`));
  await page.goto(`${base}/?example=${example}`);
  await ready(page);
  return page;
}

// 1. Nothing scrolls sideways on a 320 px phone, in any view
for (const example of ["bagel", "options", "mortgage"]) {
  const page = await open({ width: 320, height: 640 }, example);
  for (const tab of ["model", "grid", "yaml", "analyze", "mcp", "files"]) {
    await page.click(`.tab[data-tab="${tab}"]`);
    await page.waitForTimeout(150);
    const [scroll, client] = await page.evaluate(() => [document.documentElement.scrollWidth, document.documentElement.clientWidth]);
    if (scroll > client) fail(`${example}, ${tab} at 320 px: the page is ${scroll} px wide in a ${client} px window`);
  }
  await page.close();
}

// 2. The wide layout
{
  const page = await open({ width: 1440, height: 900 }, "bagel");
  const visible = (sel) => page.isVisible(sel);
  if (!(await visible("#yaml-editor")) || !(await visible('[data-scalar="year.profit"]'))) fail("1440 px: the editor and the cards are not both on screen");
  if (await visible('.tab[data-tab="yaml"]')) fail("1440 px: the YAML tab is shown although the editor is always there");

  // Typing a number in the editor changes its card
  const profitBefore = await value(page, "year.profit");
  let before = await generation(page);
  const text = await page.inputValue("#yaml-editor");
  // The price is written plain or with its format: (ADR-061)
  const price = /(price_per_dozen: (?:\{value: )?)16\b/;
  if (!price.test(text)) fail("bagel.yaml no longer has price_per_dozen: 16; update this test");
  await page.click("#yaml-editor");
  await page.fill("#yaml-editor", text.replace(price, "$117"));
  await settle(page, before);
  if ((await value(page, "assumptions.price_per_dozen")) !== "17") fail("1440 px: typing 17 in the editor did not change the price card");
  if ((await value(page, "year.profit")) === profitBefore) fail("1440 px: typing 17 in the editor did not change year.profit");

  // A change on a card changes the editor
  before = await generation(page);
  await page.fill('[data-input="assumptions.open_days_per_month"]', "28");
  await settle(page, before);
  if (!/open_days_per_month: 28\b/.test(await page.inputValue("#yaml-editor"))) fail("1440 px: a change on a card did not show in the editor");

  // The divider moves; either side can be hidden
  const box = await page.locator("#divider").boundingBox();
  await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await page.mouse.down();
  await page.mouse.move(box.x + 200, box.y + box.height / 2, { steps: 5 });
  await page.mouse.up();
  const moved = await page.locator("#divider").boundingBox();
  if (Math.abs(moved.x - box.x) < 150) fail(`1440 px: dragging the divider moved it ${Math.round(moved.x - box.x)} px, not about 200`);
  await page.click("#hide-editor");
  if (await visible("#yaml-editor")) fail("1440 px: Hide editor left the editor on screen");
  await page.click("#hide-editor");
  await page.click("#hide-cards");
  if (await visible('[data-scalar="year.profit"]')) fail("1440 px: Hide cards left the cards on screen");
  if (!(await visible("#yaml-editor"))) fail("1440 px: Hide cards hid the editor");
  await page.click("#hide-cards");
  await page.close();
}

// 3. Screenshots for the pull request
if (shots) {
  mkdirSync(shots, { recursive: true });
  for (const scheme of ["light", "dark"]) {
    for (const [name, viewport] of [["phone", { width: 390, height: 844 }], ["wide", { width: 1440, height: 900 }]]) {
      const page = await open(viewport, "bagel", scheme);
      await page.screenshot({ path: join(shots, `${name}-${scheme}.png`) });
      await page.close();
    }
  }
}

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Layouts: no sideways scroll at 320 px; at 1440 px the editor and the cards follow each other, the divider moves, either side hides.${shots ? ` Screenshots in ${shots}.` : ""}`);
