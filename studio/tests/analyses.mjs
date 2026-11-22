// The analyses that are models of their own (ADR-076), in a headless browser:
// every file of every example set whose sections Forge's `model` names as a
// decision tree, real options, a bootstrap or a Bayesian network opens on a
// card drawn from its tool's answer, not on an empty page:
//
// - every number on the card is the tool's, at the place in the answer the
//   card says it shows (a JSON pointer), in full;
// - each card shows its main figures: a tree's expected value and best
//   choice, the value with the options and without, the estimate and each
//   interval, every state of every node of a network;
// - on a network, a state tapped is evidence: the chances are then the
//   tool's given that state, and a second tap takes it back;
// - no card makes the page scroll sideways at 390 px.
//
//   node studio/tests/analyses.mjs <built Studio dir or address>

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
const TOOLS = { decision_tree: "decision_tree", real_options: "real_options", bootstrap: "bootstrap", bayesian_network: "bayesian" };
// The pointers a card must show, by section
const MUST = {
  decision_tree: ["/root_expected_value", "/risk_profile/probability_positive"],
  real_options: ["/project_value_with_options", "/traditional_npv"],
  bootstrap: ["/original_estimate", "/confidence_intervals/0/lower", "/confidence_intervals/0/upper"],
  bayesian_network: [],
};
const at = (answer, pointer) => pointer.split("/").slice(1).reduce((o, k) => (o == null ? o : o[k.replace(/~1/g, "/").replace(/~0/g, "~")]), answer);

const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
await page.addInitScript(() => localStorage.setItem("forge-studio-welcome-seen", "1"));
page.on("pageerror", (e) => fail(`page error: ${e.message}`));
await page.goto(`${base}/`);
await page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });

// Every file of every set whose sections are one of these, by Forge's `model`
const found = await page.evaluate(async (tools) => {
  const { call } = await import("./forge.js");
  const sets = await (await fetch("examples.json")).json();
  const list = [];
  for (const set of sets) {
    for (const file of Object.keys(set.files)) {
      const m = await call("model", { files: set.files, main: file }).catch(() => null);
      for (const section of m ? m.analyses.filter((a) => a in tools) : []) list.push({ set: set.id, file, section, files: set.files });
    }
  }
  return list;
}, TOOLS);
if (found.length < 9) fail(`only ${found.length} analysis files found in the example sets`);

// The numbers on a card against the tool's answer
async function compare(label, answer, section) {
  const shown = await page.$$eval("[data-analysis] [data-num]", (els) => els.map((e) => [e.dataset.num, e.dataset.value]));
  if (!shown.length) fail(`${label}: the card shows no number`);
  for (const [pointer, value] of shown) {
    const want = at(answer, pointer);
    if (String(want) !== value) fail(`${label}: ${pointer} shows ${value}, Forge answers ${want}`);
  }
  for (const pointer of MUST[section]) if (!shown.some(([p]) => p === pointer)) fail(`${label}: the card does not show ${pointer}`);
  return shown.length;
}
const answer = (item, extra = {}) => page.evaluate(async ({ item, tool, extra }) => {
  const { call } = await import("./forge.js");
  return call(tool, { files: item.files, main: item.file, ...extra });
}, { item, tool: TOOLS[item.section], extra });
const drawn = () => page.waitForFunction(() => document.body.dataset.state === "ready" && document.querySelector("[data-analysis] [data-num], [data-analysis] .msg.bad"), null, { timeout: 120000 });

let numbers = 0, evidence = 0;
for (const item of found) {
  const label = `${item.set}/${item.file}`;
  await page.goto(`${base}/?example=${item.set}&model=${item.file}`);
  try { await drawn(); } catch (e) { fail(`${label}: no card was drawn`); continue; }
  const bad = await page.$("[data-analysis] .msg.bad");
  if (bad) { fail(`${label}: ${await bad.textContent()}`); continue; }
  numbers += await compare(label, await answer(item), item.section);
  const wide = await page.evaluate(() => document.documentElement.scrollWidth);
  if (wide > 390) fail(`${label}: at 390 px the card makes the page ${wide} px wide`);
  if (item.section !== "bayesian_network") continue;
  // Every state of every node, then one seen, then taken back
  const plain = await answer(item);
  const states = await page.$$eval("[data-ev]", (els) => els.map((e) => `${e.dataset.ev}=${e.dataset.state}`));
  const all = Object.values(plain.queries).flatMap((q) => q.states.map((s) => `${q.name}=${s}`));
  if (JSON.stringify([...states].sort()) !== JSON.stringify([...all].sort())) fail(`${label}: the states on screen are ${states.join(", ")}, not ${all.join(", ")}`);
  const [node, state] = states[0].split("=");
  let before = Number(await page.evaluate(() => document.body.dataset.generation));
  await page.click(`[data-ev="${node}"][data-state="${state}"]`);
  await page.waitForFunction((g) => Number(document.body.dataset.generation) > g && document.body.dataset.state === "ready", before, { timeout: 60000 });
  const given = await answer(item, { evidence: [`${node}=${state}`] });
  numbers += await compare(`${label} given ${node}=${state}`, given, item.section);
  if ((await page.getAttribute(`[data-ev="${node}"][data-state="${state}"]`, "aria-pressed")) !== "true") fail(`${label}: ${node}=${state} is not pressed once seen`);
  if (await page.$(`[data-node="${node}"] [data-num]`)) fail(`${label}: ${node}, seen, still shows chances`);
  before = Number(await page.evaluate(() => document.body.dataset.generation));
  await page.click(`[data-ev="${node}"][data-state="${state}"]`);
  await page.waitForFunction((g) => Number(document.body.dataset.generation) > g && document.body.dataset.state === "ready", before, { timeout: 60000 });
  numbers += await compare(`${label} with nothing seen again`, plain, item.section);
  evidence++;
}

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Analyses: ${found.length} files of the example sets open on a card drawn from their tool (${[...new Set(found.map((f) => f.section))].join(", ")}), ${numbers} numbers each Forge's, and on ${evidence} Bayesian networks a state tapped is evidence, and tapped again is not.`);
