// Help in Forge Studio (ADR-077):
//
// - every entry of studio/help.json has a title, a text and a link to the
//   manual, and the link names a document of this repository and a heading
//   in it, by GitHub's rule for anchors (a heading renamed in the docs fails
//   here, not for a reader); every example it offers is a model of a set;
// - in a headless browser, every entry is reachable: a "?" for it is on a
//   heading of the Model view (a plan, the price war, the four other
//   analyses) or of Analyze, and there is no "?" without an entry;
// - a "?" opens its sheet: the title, and the manual at the release that is
//   running (github.com/…/blob/v<version>/<doc>); an example in the sheet
//   opens that model;
// - while a formula is typed on a card, the line under the suggestions is
//   Forge's syntax and description of the first function offered; while a
//   key is typed in the YAML editor, the schema's description of the first
//   key offered.
//
//   node studio/tests/help.mjs <built Studio dir or address>

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync } from "node:fs";
import { join, extname, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";
import { SETS, QUESTIONS } from "../build.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const repo = join(here, "..", "..");
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
const failures = [];
const fail = (m) => failures.push(m);

// GitHub's anchors of a Markdown file's headings: lower case, letters,
// digits, spaces, hyphens and underscores kept, spaces made hyphens, a
// repeated heading numbered; headings inside fenced code are not headings
function anchors(md) {
  const out = new Set(), seen = new Map();
  let fence = false;
  for (const line of md.split("\n")) {
    if (/^\s*(```|~~~)/.test(line)) { fence = !fence; continue; }
    const m = !fence && /^#{1,6}\s+(.*?)\s*#*\s*$/.exec(line);
    if (!m) continue;
    const slug = m[1].replace(/<[^>]+>/g, "").toLowerCase().replace(/[^\p{L}\p{N}\s_-]/gu, "").replace(/\s/g, "-");
    const n = seen.get(slug) || 0;
    seen.set(slug, n + 1);
    out.add(n ? `${slug}-${n}` : slug);
  }
  return out;
}

// The entries, without a browser
const HELP = JSON.parse(readFileSync(join(here, "..", "help.json"), "utf8"));
for (const [id, e] of Object.entries(HELP)) {
  if (!e.title || !e.text || !e.doc) { fail(`${id}: an entry needs a title, a text and a doc`); continue; }
  const [path, hash] = e.doc.split("#");
  const file = join(repo, path);
  if (!existsSync(file)) fail(`${id}: ${path} is not in the repository`);
  else if (hash && !anchors(readFileSync(file, "utf8")).has(hash)) fail(`${id}: ${path} has no heading #${hash}`);
  for (const [set, model] of e.examples || []) {
    const s = SETS.find((x) => x.id === set);
    if (!s || !s.files.includes(model) || !(QUESTIONS[set] || [[s.main]]).some(([f]) => f === model)) fail(`${id}: the example ${set}/${model} is not a model of a set`);
  }
}

const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
await page.addInitScript(() => localStorage.setItem("forge-studio-welcome-seen", "1"));
page.on("pageerror", (e) => fail(`page error: ${e.message}`));
const ready = () => page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });
const helpIds = () => page.$$eval("[data-help]", (els) => els.map((e) => e.dataset.help));
const seen = new Set();
// The Model views that have every kind of heading, and Analyze
for (const model of ["bagel.yaml", "price_war.yaml", "second_shop.yaml", "wait_option.yaml", "first_month.yaml", "slow_week.yaml"]) {
  await page.goto(`${base}/?example=bagel&model=${model}`);
  await ready();
  await page.waitForFunction(() => !document.querySelector("[data-help-slot]") && !/Forge is working/.test(document.querySelector("#view-model").textContent), null, { timeout: 60000 });
  (await helpIds()).forEach((id) => seen.add(id));
}
await page.goto(`${base}/?example=bagel`);
await ready();
await page.click('.tab[data-tab="analyze"]');
(await helpIds()).forEach((id) => seen.add(id));
for (const id of Object.keys(HELP)) if (!seen.has(id)) fail(`no "?" opens the help for ${id}`);
for (const id of seen) if (!HELP[id]) fail(`a "?" names ${id}, which help.json does not have`);

// A sheet, its link at the release, and an example opened from it
await page.goto(`${base}/?example=bagel&model=price_war.yaml`);
await ready();
const version = (await page.textContent("#status-text")).replace(/^Forge /, "");
await page.click('#games [data-help="games"]');
if ((await page.textContent("#sheet-title")) !== HELP.games.title) fail(`the games' "?" opens a sheet titled ${await page.textContent("#sheet-title")}`);
const href = await page.getAttribute("#sheet .doc", "href");
const want = `https://github.com/mollendorff-ai/forge/blob/v${version}/${HELP.games.doc}`;
if (href !== want) fail(`the games' manual link is ${href}, not ${want}`);
const [set, model] = HELP.games.examples[0];
await page.click(`#sheet [data-example="${set}"][data-example-model="${model}"]`);
await page.waitForFunction((m) => document.querySelector("#model-title").textContent === m && document.body.dataset.state === "ready", model, { timeout: 60000 });
if (!(await page.locator("#sheet").isHidden())) fail("the sheet stays open after an example is opened from it");

// A function's syntax and description while a formula is typed
await page.goto(`${base}/?example=bagel`);
await ready();
const functions = await page.evaluate(async () => {
  const { call } = await import("./forge.js");
  return (await call("functions", {})).categories.flatMap((c) => c.functions);
});
await page.click('[data-edit="year.profit"]');
const input = page.locator('[data-finput="year.profit"]');
await input.fill("=SUMI");
await input.dispatchEvent("input");
const card = page.locator('[data-fslot="year.profit"]');
await card.locator("[data-sug-help]").waitFor({ timeout: 30000 }).catch(() => {});
const first = (await card.locator(".sug.function").first().textContent().catch(() => "")).replace(/\($/, "");
const f = functions.find((x) => x.name === first);
const line = await card.locator("[data-sug-help]").textContent().catch(() => null);
if (!f) fail(`typing =SUMI offers ${first || "no function"}`);
else if (line !== `${f.syntax} · ${f.description}`) fail(`under =SUMI the help line is ${JSON.stringify(line)}, not Forge's ${JSON.stringify(`${f.syntax} · ${f.description}`)}`);

// A key's description while a key is typed in the YAML editor
await page.click('.tab[data-tab="yaml"]');
const editor = page.locator("#yaml-editor");
await editor.fill('_forge_version: "5.1.0"\nassumptions:\n  hours:\n    un');
await editor.evaluate((el) => { el.setSelectionRange(el.value.length, el.value.length); });
await editor.dispatchEvent("input");
const strip = page.locator("#view-yaml [data-suggest]");
await strip.locator("[data-sug-help]").waitFor({ timeout: 30000 }).catch(() => {});
const keyTitle = await strip.locator(".sug.key").first().getAttribute("title").catch(() => null);
const keyLine = await strip.locator("[data-sug-help]").textContent().catch(() => null);
if (!keyTitle || keyLine !== keyTitle) fail(`typing "un" under a scalar shows ${JSON.stringify(keyLine)}, not the schema's description ${JSON.stringify(keyTitle)}`);

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Help: ${Object.keys(HELP).length} entries, each linked to a heading of the manual that exists, each opened by a "?" on a heading; a sheet links the manual at Forge ${version}'s tag and opens its example; Forge's own syntax and description under a function and a key as they are typed.`);
