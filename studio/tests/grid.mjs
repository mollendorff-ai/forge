// The grid view (roadmap S11), in a headless browser with Forge's own answers:
//
// - for every table of every example, every cell of the grid is the value
//   Forge calculates for it (the number as Forge returned it, in full), the
//   grid has exactly the rows and columns of the table, and each column's
//   heading carries the formula Forge reports;
// - every scalar is in the list with its value and formula;
// - with a scenario applied the grid shows that scenario's numbers;
// - a tap on a formula cell shows the formula Forge reports, and "Show its
//   card" opens the Model tab at that card;
// - nothing in the grid can be edited.
//
//   node studio/tests/grid.mjs <built Studio dir or address>

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

// What the grid shows against what Forge returns, for one open example
async function compare(page, id, scenario) {
  return page.evaluate(async ({ id, scenario }) => {
    const { call } = await import("./forge.js");
    const sets = await (await fetch("examples.json")).json();
    const set = sets.find((x) => x.id === id);
    const request = { files: set.files, main: set.main, scenario };
    const model = await call("model", request);
    const calc = await call("calculate", request);
    const problems = [];
    let cells = 0, headings = 0, scalars = 0;
    const text = (el) => (el ? el.textContent : null);
    for (const t of model.tables) {
      const values = calc.tables[t.name] ? calc.tables[t.name].values : {};
      const rows = Math.max(0, ...Object.values(values).map((x) => x.length));
      const grid = [...document.querySelectorAll(`#view-grid [data-cell^="${t.name}."]`)].filter((el) => el.dataset.cell.slice(0, t.name.length + 1) === t.name + ".");
      if (grid.length !== rows * t.columns.length) problems.push(`${id}: ${t.name} has ${grid.length} cells, Forge has ${rows} rows of ${t.columns.length} columns`);
      for (const c of t.columns) {
        const head = document.querySelector(`#view-grid [data-gcol="${t.name}.${c.name}"]`);
        if (!head) { problems.push(`${id}: no heading for ${t.name}.${c.name}`); continue; }
        headings++;
        const f = text(head.querySelector(".g-formula"));
        if ((c.formula ?? null) !== f) problems.push(`${id}: ${t.name}.${c.name}: heading formula ${f}, Forge ${c.formula ?? null}`);
        for (let i = 0; i < rows; i++) {
          const cell = document.querySelector(`#view-grid [data-cell="${t.name}.${c.name}"][data-row="${i}"]`);
          const v = (values[c.name] || [])[i];
          const want = v === undefined || v === null ? "" : String(v);
          if (!cell) problems.push(`${id}: no cell ${t.name}.${c.name} row ${i}`);
          else if (cell.dataset.value !== want) problems.push(`${id}: ${t.name}.${c.name} row ${i}: ${cell.dataset.value}, Forge ${want}`);
          else cells++;
        }
      }
    }
    for (const s of model.scalars) {
      const row = document.querySelector(`#view-grid [data-gscalar="${s.path}"]`);
      if (!row) { problems.push(`${id}: no row for ${s.path}`); continue; }
      const v = calc.scalars[s.path];
      const want = v === undefined || v === null ? "" : String(v);
      if (row.dataset.value !== want) problems.push(`${id}: ${s.path}: ${row.dataset.value}, Forge ${want}`);
      const f = text(row.querySelector(".g-formula"));
      if ((s.formula ?? null) !== f) problems.push(`${id}: ${s.path}: formula ${f}, Forge ${s.formula ?? null}`);
      scalars++;
    }
    if (document.querySelector("#view-grid input, #view-grid textarea, #view-grid [contenteditable]")) problems.push(`${id}: the grid has something to edit`);
    return { problems, cells, headings, scalars };
  }, { id, scenario });
}

const page0 = await browser.newPage();
await page0.goto(`${base}/`);
const sets = await page0.evaluate(async () => (await (await fetch("examples.json")).json()).map((s) => s.id));
await page0.close();

let cells = 0, headings = 0, scalars = 0;
for (const id of sets) {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  page.on("pageerror", (e) => fail(`${id}: ${e.message}`));
  await page.goto(`${base}/?example=${id}`);
  await ready(page);
  await page.click('.tab[data-tab="grid"]');
  const r = await compare(page, id, null);
  r.problems.forEach(fail);
  cells += r.cells; headings += r.headings; scalars += r.scalars;
  await page.close();
}

// A scenario, a tap on a cell, and the card it leads to, on bagel
{
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  page.on("pageerror", (e) => fail(`bagel: ${e.message}`));
  await page.goto(`${base}/?example=bagel`);
  await ready(page);
  const before = await page.getAttribute('[data-scalar="year.profit"]', "data-value");
  await page.click('[data-scn="rainy_year"]');
  await page.waitForFunction((b) => document.querySelector('[data-scalar="year.profit"]').dataset.value !== b, before, { timeout: 120000 });
  await ready(page);
  await page.click('.tab[data-tab="grid"]');
  const r = await compare(page, "bagel", "rainy_year");
  r.problems.forEach((p) => fail(`rainy_year: ${p}`));
  if (!/rainy_year/.test(await page.textContent("#view-grid .lead"))) fail("the grid does not say which scenario is applied");

  // The formula of a cell, as Forge reports it
  const want = await page.evaluate(async () => {
    const { call } = await import("./forge.js");
    const sets = await (await fetch("examples.json")).json();
    const set = sets.find((x) => x.id === "bagel");
    const model = await call("model", { files: set.files, main: set.main });
    return model.tables.find((t) => t.name === "months").columns.find((c) => c.name === "revenue").formula;
  });
  await page.click('[data-cell="months.revenue"][data-row="3"]');
  const shown = await page.textContent("#sheet-body [data-gformula]");
  if (shown !== want) fail(`a tap on months.revenue shows "${shown}", Forge reports "${want}"`);
  if (!(await page.textContent("#sheet-title")).startsWith("months.revenue · row 4")) fail(`the tap's title is "${await page.textContent("#sheet-title")}"`);
  await page.click("[data-gocard]");
  await page.waitForSelector("#view-model:not([hidden])");
  const inView = await page.evaluate(() => {
    const r = document.querySelector('#view-model [data-card="months.revenue"]').getBoundingClientRect();
    return r.top >= 0 && r.bottom <= innerHeight;
  });
  if (!inView) fail("Show its card did not bring months.revenue into view");
  // A column typed into the file has no formula
  await page.click('.tab[data-tab="grid"]');
  await page.click('[data-cell="months.season"][data-row="0"]');
  if (await page.locator("#sheet-body [data-gformula]").count()) fail("an input column shows a formula");
  await page.close();
}

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.slice(0, 40).join("\n") + (failures.length > 40 ? `\n… and ${failures.length - 40} more` : ""));
  process.exit(1);
}
console.log(`Grid: ${cells} cells in ${headings} columns and ${scalars} scalars of ${sets.length} example sets are what Forge returns, with its formulas; a scenario changes them as Forge does; a tap shows the formula and leads to the card.`);
