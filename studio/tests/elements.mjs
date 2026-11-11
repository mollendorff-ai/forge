// Adding and deleting model elements on the cards (roadmap S9), in a headless
// browser, on every example:
//
// - every group of scalars gets an input and a formula, and every table a
//   column, through its Add button; each time the file changes by the new
//   lines only, and deleting the element again on its card gives back the
//   original file byte for byte;
// - a name already in the model, a formula Forge refuses, and the deletion of
//   an input other formulas use are refused with a message, and the file is
//   as it was;
// - an element the file already has is deleted with its lines and nothing
//   else (the same lines text.js's deleteEntryText removes).
//
//   node studio/tests/elements.mjs <built Studio dir or address>

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";
import { deleteEntryText } from "../text.js";

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
let added = 0, deleted = 0, refused = 0;

const settled = (page) => page.waitForFunction(() => document.body.dataset.state !== "loading", null, { timeout: 120000 });
async function filesOf(page, set) {
  const saved = await page.evaluate(() => JSON.parse(localStorage.getItem("forge-studio-v3") || "null"));
  return saved ? saved.files : set.files;
}
const q = (s) => s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');

// Fill the Add form of `key` and press Check and add; answers the message
// shown, or null when the element was added
async function add(page, key, kind, name, value) {
  await page.click(`[data-add="${q(key)}"]`);
  const slot = page.locator(`[data-addslot="${q(key)}"]`);
  if (kind === "formula") await slot.locator('[data-akind="formula"]').click();
  await slot.locator("[data-aname]").fill(name);
  await slot.locator(kind === "input" ? "[data-ainput]" : "[data-aform]").fill(String(value));
  await slot.locator("[data-asave]").click();
  await page.waitForFunction((k) => {
    const s = document.querySelector(`[data-addslot="${k}"]`);
    const m = s && s.querySelector("[data-amsg]");
    return (m && !m.hidden) || (s && s.hidden && document.body.dataset.state !== "loading");
  }, q(key), { timeout: 120000 });
  const msg = await page.evaluate((k) => { const m = document.querySelector(`[data-addslot="${k}"] [data-amsg]`); return m && !m.hidden ? m.textContent : null; }, q(key));
  if (msg) await page.click(`[data-addslot="${q(key)}"] [data-acancel]`);
  return msg;
}
// Delete `path` on its card; answers Forge's message, or null when deleted
async function del(page, path) {
  const button = page.locator(`[data-del="${q(path)}"]`);
  if (!(await button.count())) return `no Delete button for ${path}`;
  if (!(await button.isVisible())) await page.click(`[data-col="${q(path)}"]`);
  await button.click();
  await page.click(`[data-delyes="${q(path)}"]`);
  await page.waitForFunction((p) => !document.querySelector("#sheet").hidden
    || (!document.querySelector(`[data-del="${p}"]`) && document.body.dataset.state !== "loading"), q(path), { timeout: 120000 });
  const sheet = await page.evaluate(() => (document.querySelector("#sheet").hidden ? null : document.querySelector("#sheet-body").textContent));
  if (sheet) await page.click("#sheet-close");
  return sheet;
}
// `after` is `before` with lines inserted in one place, the first of them
// starting with `name:`
function onlyInserted(before, after, name) {
  const a = before.split("\n"), b = after.split("\n");
  let i = 0;
  while (i < a.length && a[i] === b[i]) i++;
  const n = b.length - a.length;
  if (n <= 0) return false;
  return b.slice(i + n).join("\n") === a.slice(i).join("\n") && new RegExp(`^\\s*${name}:`).test(b[i]);
}

const page0 = await browser.newPage();
await page0.goto(`${base}/`);
const sets = await page0.evaluate(async () => (await fetch("examples.json")).json());
await page0.close();

for (const set of sets) {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  page.on("pageerror", (e) => fail(`${set.id}: ${e.message}`));
  await page.goto(`${base}/?example=${set.id}`);
  await settled(page);
  const buttons = await page.$$eval("[data-add]", (els) => els.map((b) => ({ ...b.dataset })));
  if (!buttons.length) fail(`${set.id}: no Add button`);
  for (const b of buttons) {
    const file = b.ns ? (await page.evaluate((ns) => [...document.querySelectorAll(".eyebrow .inc")].map((x) => x.textContent).find((t) => t.startsWith(`@${ns} `)), b.ns)).split(" · ")[1] : set.main;
    const prefix = `${b.ns ? "@" + b.ns + "." : ""}${b.group ? b.group + "." : ""}`;
    const tries = b.kind === "column" ? [["column", "s9_column", "=ROW()"]] : [["input", "s9_input", "1.5"], ["formula", "s9_formula", "=1 + 1"]];
    for (const [kind, name, value] of tries) {
      const what = `${set.id}: ${kind} ${prefix}${name}`;
      const before = (await filesOf(page, set))[file];
      const msg = await add(page, b.add, kind, name, value);
      if (msg) { fail(`${what} was not added: ${msg}`); continue; }
      const after = (await filesOf(page, set))[file];
      if (!onlyInserted(before, after, name)) { fail(`${what}: the file changed by more than the new lines`); continue; }
      added++;
      const dmsg = await del(page, prefix + name);
      if (dmsg) { fail(`${what} was not deleted: ${dmsg}`); continue; }
      if ((await filesOf(page, set))[file] !== before) fail(`${what}: deleting it did not give back the file byte for byte`);
      else deleted++;
    }
  }
  await page.close();
}

// Refusals and an existing element, on the bagel example
const bagel = sets.find((s) => s.id === "bagel");
const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
page.on("pageerror", (e) => fail(`bagel: ${e.message}`));
await page.goto(`${base}/?example=bagel`);
await settled(page);
const original = (await filesOf(page, bagel))["bagel.yaml"];
const unchanged = async (what) => {
  if ((await filesOf(page, bagel))["bagel.yaml"] !== original) fail(`${what}: the file changed`);
  else refused++;
};
let msg = await add(page, "assumptions", "input", "price_per_dozen", "3");
if (!msg || !/already in the model/.test(msg)) fail(`an existing name was not refused (${msg})`);
await unchanged("an existing name");
msg = await add(page, "year", "formula", "s9_bad", "=nope_s9 + 1");
if (!msg) fail("a formula Forge refuses was added");
await unchanged("a formula Forge refuses");
msg = await del(page, "assumptions.price_per_dozen");
if (!msg) fail("an input other formulas use was deleted");
await unchanged("deleting an input other formulas use");
msg = await del(page, "year.first_year_return");
const want = deleteEntryText(original, "year", "first_year_return");
if (msg) fail(`year.first_year_return was not deleted: ${msg}`);
else if ((await filesOf(page, bagel))["bagel.yaml"] !== want) fail("deleting year.first_year_return changed more than its lines");
else deleted++;
await page.close();

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Elements: ${added} added and ${deleted} deleted on the cards of ${sets.length} example sets, each the new lines only and back to the file byte for byte; ${refused} refusals leave the file as it was.`);
