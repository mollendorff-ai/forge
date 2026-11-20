// Forge Studio's games (ADR-068), in a headless browser, on the bagel's price
// war (examples/bagel/price_war.yaml, opened from the bagel set's Files tab):
//
// - every cell of the payoff table holds the payoffs the `game` tool answers,
//   and the cells marked as an equilibrium are exactly Forge's pure
//   equilibria: (cut, cut) in the plan, (hold, hold) with loyal regulars;
// - the tournament's ranking is Forge's, in Forge's order and with its
//   scores, the stable players are Forge's, and every match is Forge's;
// - an input typed on a card moves the games: 0.55 for the share a cheaper
//   shop takes is the loyal_regulars scenario, and gives its equilibrium;
// - the Tools tab runs forge_game on the open model.
//
//   node studio/tests/games.mjs <built Studio dir or address>

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
const ready = (page) => page.waitForFunction(() => document.body.dataset.state !== "loading", null, { timeout: 120000 });
const generation = (page) => page.evaluate(() => Number(document.body.dataset.generation));
const settle = (page, before) => page.waitForFunction((g) => Number(document.body.dataset.generation) > g && document.body.dataset.state !== "loading", before, { timeout: 60000 });

const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
page.on("pageerror", (e) => fail(`price war: ${e.message}`));
await page.goto(`${base}/?example=bagel`);
await ready(page);
await page.click('.tab[data-tab="files"]');
await page.click('[data-main="price_war.yaml"]');
await ready(page);
await page.waitForSelector('[data-game="price_war"] table');

// Forge's own answer for the files the page has open
const forge = (scenario) => page.evaluate(async (scenario) => {
  const { call } = await import("./forge.js");
  const set = (await (await fetch("examples.json")).json()).find((x) => x.id === "bagel");
  const files = { ...set.files };
  return call("game", { files, main: "price_war.yaml", scenario });
}, scenario);

// What the page shows: the table's cells, the ranking, the stable players
// and the matches, from the data- attributes the cards keep
const shown = () => page.evaluate(() => {
  const table = document.querySelector('[data-game="price_war"]');
  const tour = document.querySelector('[data-game="rivalry"]');
  return {
    cells: [...table.querySelectorAll("[data-cell]")].map((td) => ({ cell: td.dataset.cell, pay: td.dataset.pay, eq: td.dataset.eq === "1" })),
    ranking: [...tour.querySelectorAll("[data-standing]")].map((tr) => ({ player: tr.dataset.standing, score: tr.dataset.score })),
    stable: tour.querySelector("[data-stable]").textContent,
    matches: Object.fromEntries([...tour.querySelectorAll("[data-match]")].map((td) => [td.dataset.match, td.dataset.pay])),
  };
});

// The page against Forge, for one state of the model; `pure` is the cell
// the equilibrium must be in, as worked out in src/cli/commands/examples
async function check(label, scenario, pure) {
  const [want, got] = [await forge(scenario), await shown()];
  const [table, tour] = want;
  const eqCells = new Set(table.equilibria.filter((e) => e.kind === "pure")
    .map((e) => table.players.map((p) => Object.entries(e.players[p].strategies).find(([, x]) => x === 1)[0]).join(",")));
  if (got.cells.length !== table.cells.length) fail(`${label}: ${got.cells.length} cells on screen, Forge has ${table.cells.length}`);
  for (const c of table.cells) {
    const key = c.strategies.join(","), on = got.cells.find((x) => x.cell === key);
    if (!on) { fail(`${label}: no cell ${key} on screen`); continue; }
    if (on.pay !== c.payoffs.join(",")) fail(`${label}: cell ${key} shows ${on.pay}, Forge gives ${c.payoffs.join(",")}`);
    if (on.eq !== eqCells.has(key)) fail(`${label}: cell ${key} is ${on.eq ? "" : "not "}marked as an equilibrium, against Forge`);
  }
  if ([...eqCells].join(";") !== pure) fail(`${label}: Forge's pure equilibria are ${[...eqCells].join(";")}, not ${pure}`);
  const order = tour.ranking.map((s) => s.player).join(",");
  if (got.ranking.map((s) => s.player).join(",") !== order) fail(`${label}: the ranking on screen is ${got.ranking.map((s) => s.player)}, Forge's is ${order}`);
  for (const s of tour.ranking) {
    const on = got.ranking.find((x) => x.player === s.player);
    if (!on || on.score !== String(s.score)) fail(`${label}: ${s.player} scores ${on && on.score} on screen, Forge gives ${s.score}`);
  }
  if (!got.stable.includes(`: ${tour.stable.length ? tour.stable.join(", ") : "none"}.`)) fail(`${label}: the stable players on screen are "${got.stable}", Forge's are ${tour.stable}`);
  for (const [a, row] of Object.entries(tour.matches)) {
    for (const [b, v] of Object.entries(row)) {
      if (got.matches[`${a},${b}`] !== String(v)) fail(`${label}: ${a} against ${b} shows ${got.matches[`${a},${b}`]}, Forge gives ${v}`);
    }
  }
  return tour;
}

// The plan: both cut; matching, grudges and the discounter are stable
const plan = await check("plan", null, "cut,cut");
if (plan.stable.join(",") !== "matcher,grudge,discounter") fail(`plan: Forge's stable players are ${plan.stable}`);

// Each scenario: tap its chip and read the games again
let before = await generation(page);
await page.click('[data-scn="loyal_regulars"]');
await settle(page, before);
const loyal = await check("loyal_regulars", "loyal_regulars", "hold,hold");
if (loyal.ranking[loyal.ranking.length - 1].player !== "discounter") fail("loyal_regulars: the discounter does not come last");
before = await generation(page);
await page.click('[data-scn="lease_ending"]');
await settle(page, before);
const ending = await check("lease_ending", "lease_ending", "cut,cut");
if (ending.stable.join(",") !== "discounter") fail(`lease_ending: Forge's stable players are ${ending.stable}, not the discounter alone`);

// An input typed on a card moves the games: back to the plan, then 0.55
before = await generation(page);
await page.click('[data-scn=""]');
await settle(page, before);
before = await generation(page);
await page.fill('[data-input="market.undercut_share"]', "0.55");
await settle(page, before);
const marked = await page.$$eval('[data-game="price_war"] [data-eq]', (tds) => tds.map((td) => td.dataset.cell));
if (marked.join(";") !== "hold,hold") fail(`typing 0.55 on market.undercut_share marks ${marked}, not hold,hold`);

// The Tools tab runs forge_game here
await page.click('.tab[data-tab="mcp"]');
await page.click('[data-tool="game"]');
await page.click('[data-run="game"]');
await page.waitForFunction(() => /"kind": "tournament"|msg bad/.test(document.querySelector('[data-out="game"]').innerHTML), null, { timeout: 60000 });
const answer = await page.textContent('[data-out="game"]');
if (!answer.includes('"game": "price_war"') || !answer.includes('"cells"')) fail(`the Tools tab's forge_game answered: ${answer.slice(0, 200)}`);
await page.close();

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log("Games: the bagel's price war shows Forge's payoffs, equilibria, ranking, stable players and matches in the plan and both scenarios, an input typed on a card moves them, and the Tools tab runs forge_game.");
