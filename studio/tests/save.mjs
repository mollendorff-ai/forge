// Saving from Forge Studio through the share sheet (an iPhone's Save to
// Files, iCloud Drive), in a headless browser with a share sheet that records
// what it is given:
//
// - Save on the Files tab shares the model's files, the main one and the ones
//   it includes, with their text as Studio holds it;
// - Save beside the workbook shares the workbook forge export writes (part by
//   part, apart from when it was made);
// - both call share() while the tap is still being handled, which Safari
//   requires: nothing outside the tap is awaited before it.
//
// Without a share sheet the files are downloaded; tests/excel.mjs covers that.
//
//   node studio/tests/save.mjs <built Studio dir or address> <dir of forge export's workbooks>

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";
import { differences } from "../../forge-wasm/tests/node/xlsx.mjs";

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
const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
const failures = [];
const fail = (m) => failures.push(m);
page.on("pageerror", (e) => fail(e.message));

// A share sheet that keeps what it is given, and whether it was called while
// the browser was still handling the tap. Chromium keeps a tap "active" for
// five seconds, far longer than Safari, so the test asks for more: share()
// runs before the click event has finished. Promise steps (microtasks) run
// within it; a wait for anything else, such as the worker making the
// workbook or a timer, does not, and fails this test.
await page.addInitScript(() => {
  window.__shared = [];
  window.__inTap = false;
  document.addEventListener("click", () => { window.__inTap = true; }, true);
  window.addEventListener("click", () => { window.__inTap = false; });
  Object.defineProperty(navigator, "canShare", { value: (data) => !!(data && data.files && data.files.length), configurable: true });
  Object.defineProperty(navigator, "share", {
    configurable: true,
    value: (data) => {
      const active = window.__inTap;
      const files = data.files.map(async (f) => ({ name: f.name, type: f.type, bytes: Array.from(new Uint8Array(await f.arrayBuffer())) }));
      return Promise.all(files).then((list) => { window.__shared.push({ active, files: list }); });
    },
  });
});

await page.goto(`${base}/?example=bagel`);
await page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });
await page.click('.tab[data-tab="files"]');

// The model's files
await page.click("#save-yaml");
await page.waitForFunction(() => window.__shared.length === 1, null, { timeout: 30000 });
const examples = await page.evaluate(async () => (await (await fetch("examples.json")).json()).find((x) => x.id === "bagel").files);
const yaml = (await page.evaluate(() => window.__shared[0]));
if (!yaml.active) fail("Save (YAML): share() was called after the tap had ended; Safari would refuse");
const names = yaml.files.map((f) => f.name).join(", ");
if (names !== "bagel.yaml, costs.yaml") fail(`Save (YAML) shared ${names}, not bagel.yaml, costs.yaml`);
for (const f of yaml.files) {
  if (Buffer.from(f.bytes).toString("utf8") !== examples[f.name]) fail(`Save (YAML): ${f.name} is not the model's text`);
}

// The workbook, once Studio has made it (it does when the Files tab opens)
await page.waitForTimeout(3000);
await page.click("#xlsx");
await page.waitForFunction(() => window.__shared.length === 2, null, { timeout: 60000 });
const xlsx = await page.evaluate(() => window.__shared[1]);
if (!xlsx.active) fail("Save (Excel): share() was called after the tap had ended; Safari would refuse");
if (xlsx.files.length !== 1 || xlsx.files[0].name !== "bagel.xlsx") fail(`Save (Excel) shared ${xlsx.files.map((f) => f.name).join(", ")}, not bagel.xlsx`);
else {
  for (const d of differences(Buffer.from(xlsx.files[0].bytes), readFileSync(join(workbooks, "bagel.xlsx")))) fail(`Save (Excel): ${d} from forge export's workbook`);
}

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log("Save: the model's files and forge export's workbook go to the share sheet within the tap.");
