// A set's models (ADR-077), in a headless browser:
//
// - the bagel's first visit lists its nine models as questions on the welcome
//   card, in the set's order, each with the kind of model Forge says its
//   file is (the plan a calculation, its simulation a simulation, the price
//   war and the festival games, then a decision tree, a real option, a
//   bootstrap and a Bayesian network);
// - a tap on a question opens that model, and the card is then read;
// - a strip at the top of the Model view has every model of the set, the
//   open one pressed, and a tap on a chip opens that model;
// - in every set of several models, each chip opens its file, and Files says
//   each model's question;
// - at 320 px the strip does not make the page scroll sideways.
//
//   node studio/tests/sets.mjs <built Studio dir or address>

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
const ready = (page) => page.waitForFunction(() => document.body.dataset.state === "ready" || document.body.dataset.state === "error", null, { timeout: 120000 });
// Every badge on the page has Forge's answer
const kinds = (page) => page.waitForFunction(() => [...document.querySelectorAll("[data-kind-of]")].every((k) => k.textContent), null, { timeout: 60000 });
const title = (page) => page.textContent("#model-title");
const sets = await (await fetch(`${base}/examples.json`)).json();
const bagel = sets.find((s) => s.id === "bagel");

// The bagel's first visit
const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
page.on("pageerror", (e) => fail(`page error: ${e.message}`));
await page.goto(`${base}/?example=bagel`);
await ready(page);
await kinds(page);
const questions = await page.$$eval("#welcome .question", (qs) => qs.map((q) => [q.dataset.model, q.querySelector(".kind").textContent, q.querySelector("b").textContent]));
const expected = ["calculation", "simulation", "game", "game", "decision tree", "real option", "bootstrap", "Bayes", "calculation"];
if (JSON.stringify(questions.map(([file, , q]) => [file, q])) !== JSON.stringify(bagel.models.map(([file, , q]) => [file, q]))) fail(`the welcome card's questions are ${JSON.stringify(questions)}, not the set's`);
if (JSON.stringify(questions.map(([, kind]) => kind)) !== JSON.stringify(expected)) fail(`the bagel's kinds are ${questions.map(([, k]) => k).join(", ")}, not ${expected.join(", ")}`);
if (!/leaves this device/.test(await page.textContent("#welcome"))) fail("the welcome card does not say that nothing leaves the device");

// The strip, with the plan pressed
const chips = await page.$$eval(".set-strip [data-model]", (cs) => cs.map((c) => [c.dataset.model, c.getAttribute("aria-pressed")]));
if (chips.length !== bagel.models.length) fail(`the strip has ${chips.length} chips, not ${bagel.models.length}`);
if (JSON.stringify(chips.filter(([, p]) => p === "true").map(([f]) => f)) !== '["bagel.yaml"]') fail(`the strip presses ${JSON.stringify(chips.filter(([, p]) => p === "true"))}, not the plan`);

// A question opens its model, and the card is read
await page.click('#welcome [data-model="price_war.yaml"]');
await page.waitForFunction(() => document.querySelector("#model-title").textContent === "price_war.yaml" && document.body.dataset.state === "ready" && document.querySelector('[data-game="price_war"] table'), null, { timeout: 60000 });
if (await page.locator("#welcome").count()) fail("after a question was tapped, the welcome card is still there");
if ((await page.getAttribute('.set-strip [data-model="price_war.yaml"]', "aria-pressed")) !== "true") fail("the strip does not press the price war once it is open");
// and shows its chip, wherever it is along the strip
const seen = await page.evaluate(() => { const strip = document.querySelector(".set-strip"), on = strip.querySelector('[aria-pressed="true"]').getBoundingClientRect(), box = strip.getBoundingClientRect(); return on.left >= box.left - 1 && on.right <= box.right + 1; });
if (!seen) fail("the price war's chip is pressed but scrolled out of the strip");

// A chip opens its model
await page.click('.set-strip [data-model="bagel_simulate.yaml"]');
await page.waitForFunction(() => document.querySelector("#model-title").textContent === "bagel_simulate.yaml" && document.body.dataset.state !== "loading", null, { timeout: 60000 });
await page.reload();
await ready(page);
if (await page.locator("#welcome").count()) fail("after a reload, the welcome card came back although a question was tapped");
await page.close();

// Every set of several models: each chip opens its file, and Files names each
// model's question
let opened = 0;
for (const set of sets.filter((s) => s.models)) {
  const p = await browser.newPage({ viewport: { width: 390, height: 844 } });
  await p.addInitScript(() => localStorage.setItem("forge-studio-welcome-seen", "1"));
  p.on("pageerror", (e) => fail(`${set.id}: page error: ${e.message}`));
  await p.goto(`${base}/?example=${set.id}`);
  await ready(p);
  await kinds(p);
  for (const [file] of set.models) {
    await p.click(`.set-strip [data-model="${file}"]`);
    const want = file === set.main ? set.title : file;
    try {
      await p.waitForFunction(({ want, file }) => document.querySelector("#model-title").textContent === want && document.body.dataset.state !== "loading" && document.querySelector(`.set-strip [data-model="${file}"]`).getAttribute("aria-pressed") === "true", { want, file }, { timeout: 60000 });
      opened++;
    } catch (e) { fail(`${set.id}: the chip of ${file} did not open it`); }
  }
  await p.click('.tab[data-tab="files"]');
  const notes = await p.$$eval("#view-files .row.file", (rows) => rows.map((r) => r.textContent));
  for (const [file, , question] of set.models) {
    if (!notes.some((n) => n.includes(file) && (n.includes(question) || n.includes("Main model")))) fail(`${set.id}: Files does not say ${file}'s question`);
  }
  await p.close();
}

// Narrow screens
const narrow = await browser.newPage({ viewport: { width: 320, height: 700 } });
await narrow.goto(`${base}/?example=acquisition`);
await ready(narrow);
const wide = await narrow.evaluate(() => document.documentElement.scrollWidth);
if (wide > 320) fail(`at 320 px the page with the questions and the strip is ${wide} px wide`);

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Sets: the bagel's welcome card asks its ${bagel.models.length} questions with the kinds Forge gives (${expected.join(", ")}), a question opens its model, the strip opens each of ${opened} models in ${sets.filter((s) => s.models).length} sets, Files says each question, and it all fits 320 px.`);
