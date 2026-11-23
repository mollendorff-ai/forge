// "Add a game" on Forge Studio's Model view (ADR-076 decision 6), in a headless
// browser at 390 px, on the bagel (which has no games):
//
// - the templates offered are the `templates` tool's, each with Forge's line
//   and its default name as the placeholder;
// - each template added under a name gives the games `game` answers for the
//   files `edit` wrote (cells, standings, the count form's numbers), and the
//   model goes to format 5.5.0, which has games;
// - a name the model already uses is refused with Forge's own message, and
//   nothing changes.
//
//   node studio/tests/templates.mjs <built Studio dir or address>

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
let page;
async function visit() {
  if (page) await page.close();
  page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  page.on("pageerror", (e) => fail(e.message));
  await page.goto(`${base}/?example=bagel&model=bagel.yaml`);
  await page.waitForFunction(() => document.body.dataset.state !== "loading", null, { timeout: 120000 });
  await page.waitForSelector("[data-templates]");
}

// Forge's answers, for the bagel's own files with these changes made
const forge = (changes) => page.evaluate(async (changes) => {
  const { call } = await import("./forge.js");
  const set = (await (await fetch("examples.json")).json()).find((x) => x.id === "bagel");
  const main = "bagel.yaml";
  try {
    const { files } = await call("edit", { files: { ...set.files }, main, changes });
    return { files, games: await call("game", { files: { ...set.files, ...files }, main }) };
  } catch (e) { return { why: e.message }; }
}, changes);
const listed = () => page.evaluate(async () => (await import("./forge.js")).call("templates", {}));

const shown = () => page.evaluate(() => Object.fromEntries([...document.querySelectorAll("#view-model [data-game]")].map((card) => [card.dataset.game, {
  cells: [...card.querySelectorAll("[data-cell]")].map((td) => `${td.dataset.cell}=${td.dataset.pay}`),
  counts: [...card.querySelectorAll("[data-count]")].map((td) => `${td.dataset.count}=${td.dataset.pay}`),
  ranking: [...card.querySelectorAll("[data-standing]")].map((tr) => `${tr.dataset.standing}=${tr.dataset.score}`),
  dynamics: card.querySelector("[data-dynamics]") ? card.querySelector("[data-dynamics]").dataset.generations : null,
}])));
const settled = () => page.waitForFunction(() => document.body.dataset.state !== "loading"
  && [...document.querySelectorAll("#view-model [data-game]")].every((c) => !/Forge is working out/.test(c.textContent)), null, { timeout: 60000 });

await visit();
const templates = await listed();
const offered = await page.$$eval("[data-templates] [data-tsel] option", (os) => os.map((o) => o.value));
if (offered.join(",") !== templates.map((t) => t.name).join(",")) fail(`the card offers ${offered}, Forge's templates are ${templates.map((t) => t.name)}`);

for (const t of templates) {
  await visit();
  await page.selectOption("[data-templates] [data-tsel]", t.name);
  const line = await page.textContent("[data-templates] [data-tline]");
  const hint = await page.getAttribute("[data-templates] [data-tname]", "placeholder");
  if (line !== t.line || hint !== t.default) fail(`${t.name}: the card says "${line}" and "${hint}", Forge's are "${t.line}" and "${t.default}"`);
  await page.fill("[data-templates] [data-tname]", "mine");
  await page.click("[data-templates] [data-tadd]");
  await page.waitForSelector('[data-game="mine"]');
  await settled();
  const change = [{ op: "insert", path: t.name, new_name: "mine" }];
  const want = await forge(change);
  if (!want.games) { fail(`${t.name}: Forge refused: ${want.why}`); continue; }
  const got = await shown();
  if (Object.keys(got).join(",") !== want.games.map((g) => g.game).join(",")) fail(`${t.name}: the cards are ${Object.keys(got)}, Forge's games are ${want.games.map((g) => g.game)}`);
  for (const g of want.games) {
    const on = got[g.game];
    if (!on) continue;
    const cells = g.kind === "tournament" ? [] : g.cells.map((c) => `${c.strategies.join(",")}=${c.payoffs.join(",")}`);
    const ranking = g.kind === "tournament" ? g.ranking.map((s) => `${s.player}=${s.score}`) : [];
    if (on.counts.length === 0 && JSON.stringify(on.cells) !== JSON.stringify(cells)) fail(`${t.name}: ${g.game}'s cells are ${on.cells}, Forge's are ${cells}`);
    if (t.name === "game_of_many" && !on.counts.length) fail(`${t.name}: ${g.game} is not drawn by count`);
    if (JSON.stringify(on.ranking) !== JSON.stringify(ranking)) fail(`${t.name}: ${g.game}'s standings are ${on.ranking}, Forge's are ${ranking}`);
    if ((on.dynamics !== null) !== Boolean(g.dynamics)) fail(`${t.name}: ${g.game} ${on.dynamics ? "draws" : "lacks"} dynamics against Forge`);
  }
  const text = want.files["bagel.yaml"];
  if (!/_forge_version: "5\.[56]\.0"/.test(text)) fail(`${t.name}: the model is not in a format with games`);
}

// A name the model uses
await visit();
const clash = await forge([{ op: "insert", path: "game_of_two", new_name: "assumptions" }]);
await page.fill("[data-templates] [data-tname]", "assumptions");
await page.click("[data-templates] [data-tadd]");
await page.waitForSelector("[data-templates] [data-tmsg]:not([hidden])");
const message = await page.textContent("[data-templates] [data-tmsg]");
if (!clash.why) fail("Forge accepts a game named like a group of the model");
else if (message !== clash.why) fail(`the card says "${message}", Forge says "${clash.why}"`);
if (await page.$("[data-game]")) fail("a refused template added a game");
if (await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)) fail("the cards are wider than the screen");

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Templates: the ${templates.length} templates Forge serves are offered, each added to the bagel gives the games Forge works out for the files edit wrote, and a name in use is refused with Forge's message.`);
