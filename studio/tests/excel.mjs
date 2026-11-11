// Excel in the browser (roadmap S8), in a headless browser:
//
// - the workbook Forge Studio downloads for bagel and mortgage is the one
//   `forge export` writes for the same release: the same parts, in the same
//   order, each the same byte for byte, apart from the time the workbook was
//   made (docProps/core.xml, to the second), which is the only difference
//   listed and explained; when both are made in the same second the files
//   are equal byte for byte;
// - a workbook `forge export` wrote, opened in Studio, is imported by Forge
//   and calculates to the same numbers (bagel's year.profit, 786.32).
//
//   node studio/tests/excel.mjs <built Studio dir or address> <dir of forge export's workbooks>
//
// The workbooks come from `cargo run -p forge-wasm --release --example expected -- <dir>`,
// which calls the command line's export_core.

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync, appendFileSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";
import { parts, differences } from "../../forge-wasm/tests/node/xlsx.mjs";

const [where, workbooks] = process.argv.slice(2);
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
const rows = [];

for (const [example, file] of [["bagel", "bagel.xlsx"], ["mortgage", "mortgage.xlsx"]]) {
  const page = await browser.newPage({ acceptDownloads: true, viewport: { width: 390, height: 844 } });
  page.on("pageerror", (e) => fail(`${example}: ${e.message}`));
  await page.goto(`${base}/?example=${example}`);
  await page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });
  await page.click('.tab[data-tab="files"]');
  const [download] = await Promise.all([page.waitForEvent("download", { timeout: 120000 }), page.click("#xlsx")]);
  if (download.suggestedFilename() !== file) fail(`${example}: the download is named ${download.suggestedFilename()}, not ${file}`);
  const studio = readFileSync(await download.path());
  const cli = readFileSync(join(workbooks, file));
  const found = differences(studio, cli);
  for (const d of found) fail(`${example}: ${d} between Studio's workbook and forge export's`);
  const created = (bytes) => /<dcterms:created[^>]*>([^<]*)</.exec(parts(bytes).get("docProps/core.xml").toString())[1];
  rows.push(`| ${file} | ${studio.length} | ${cli.length} | ${studio.equals(cli) ? "yes" : "no"} | ${found.length ? found.join("; ") : `only the creation time (${created(studio)} against ${created(cli)})`} |`);
  await page.close();
}

// forge export's workbook, opened in Studio, imported by Forge
{
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  page.on("pageerror", (e) => fail(`import: ${e.message}`));
  await page.goto(`${base}/?example=starter`);
  await page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });
  await page.click('.tab[data-tab="files"]');
  await page.setInputFiles("#imp", join(workbooks, "bagel.xlsx"));
  await page.waitForFunction(() => document.querySelector("#model-files")?.textContent.startsWith("bagel.yaml") && document.body.dataset.state !== "loading", null, { timeout: 120000 });
  const profit = Number(await page.getAttribute('[data-scalar="year.profit"]', "data-value"));
  if (Math.abs(profit - 786.32) > 0.005) fail(`import: bagel.xlsx gives year.profit ${profit}, not 786.32`);
  const files = (await page.textContent("#model-files")).trim();
  if (files !== "bagel.yaml + costs.yaml") fail(`import: the model's files are "${files}", not bagel.yaml + costs.yaml`);
  rows.push(`| import of forge export's bagel.xlsx | | | | ${files}, year.profit ${profit.toFixed(2)} |`);
  await page.close();
}

await browser.close();
if (!remote) server.close();
const summary = ["## Excel in Forge Studio", "", "| Workbook | Studio (bytes) | forge export (bytes) | Same bytes | Difference |", "| --- | ---: | ---: | --- | --- |", ...rows].join("\n");
console.log(summary);
if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary + "\n");
if (failures.length) {
  console.error(`\n${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
