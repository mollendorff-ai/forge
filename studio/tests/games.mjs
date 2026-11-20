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
// - the Tools tab runs forge_game on the open model;
// - in the Games set (examples/games), every game of each of its seven files,
//   opened from the Files tab, is Forge's: each cell, each equilibrium with
//   its chances, each ranking, stable list and match; the stag hunt's scenario
//   c3 and a cost typed on its card move the mixed equilibrium as Forge does,
//   from 3/4 stag to (3 - c) / 4.
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

let page = await browser.newPage({ viewport: { width: 390, height: 844 } });
page.on("pageerror", (e) => fail(`price war: ${e.message}`));
await page.goto(`${base}/?example=bagel`);
await ready(page);
await page.click('.tab[data-tab="files"]');
await page.click('[data-main="price_war.yaml"]');
await ready(page);
await page.waitForSelector('[data-game="price_war"] table');

// Forge's own answer for the files of an example set, opened on `main`
const forge = (id, main, scenario) => page.evaluate(async ([id, main, scenario]) => {
  const { call } = await import("./forge.js");
  const set = (await (await fetch("examples.json")).json()).find((x) => x.id === id);
  return call("game", { files: { ...set.files }, main, scenario });
}, [id, main, scenario]);

// What one game's card shows, from the data- attributes it keeps: a table's
// cells and equilibria, or a tournament's ranking, stable players and matches
const shown = (name) => page.evaluate((name) => {
  const card = document.querySelector(`[data-game="${name}"]`);
  if (!card) return null;
  return {
    cells: [...card.querySelectorAll("[data-cell]")].map((td) => ({ cell: td.dataset.cell, pay: td.dataset.pay, eq: td.dataset.eq === "1" })),
    equilibria: [...card.querySelectorAll("[data-equilibrium]")].map((li) =>
      Object.fromEntries([...li.querySelectorAll("[data-play]")].map((s) => [s.dataset.play, s.dataset.chances]))),
    ranking: [...card.querySelectorAll("[data-standing]")].map((tr) => ({ player: tr.dataset.standing, score: tr.dataset.score })),
    stable: card.querySelector("[data-stable]") ? card.querySelector("[data-stable]").textContent : null,
    matches: Object.fromEntries([...card.querySelectorAll("[data-match]")].map((td) => [td.dataset.match, td.dataset.pay])),
  };
}, name);

// The pure equilibria of a table, as "row,column" keys
const pureCells = (table) => [...new Set(table.equilibria.filter((e) => e.kind === "pure")
  .map((e) => table.players.map((p) => Object.entries(e.players[p].strategies).find(([, x]) => x === 1)[0]).join(",")))];

// One game's card against Forge's answer for it
function compare(label, want, got) {
  if (!got) { fail(`${label}: no card for ${want.game}`); return; }
  if (want.kind === "tournament") {
    const order = want.ranking.map((s) => s.player).join(",");
    if (got.ranking.map((s) => s.player).join(",") !== order) fail(`${label}: the ranking on screen is ${got.ranking.map((s) => s.player)}, Forge's is ${order}`);
    for (const s of want.ranking) {
      const on = got.ranking.find((x) => x.player === s.player);
      if (!on || on.score !== String(s.score)) fail(`${label}: ${s.player} scores ${on && on.score} on screen, Forge gives ${s.score}`);
    }
    if (!got.stable || !got.stable.includes(`: ${want.stable.length ? want.stable.join(", ") : "none"}.`)) fail(`${label}: the stable players on screen are "${got.stable}", Forge's are ${want.stable}`);
    for (const [a, row] of Object.entries(want.matches)) {
      for (const [b, v] of Object.entries(row)) {
        if (got.matches[`${a},${b}`] !== String(v)) fail(`${label}: ${a} against ${b} shows ${got.matches[`${a},${b}`]}, Forge gives ${v}`);
      }
    }
    return;
  }
  const eqCells = new Set(pureCells(want));
  if (got.cells.length !== want.cells.length) fail(`${label}: ${got.cells.length} cells on screen, Forge has ${want.cells.length}`);
  for (const c of want.cells) {
    const key = c.strategies.join(","), on = got.cells.find((x) => x.cell === key);
    if (!on) { fail(`${label}: no cell ${key} on screen`); continue; }
    if (on.pay !== c.payoffs.join(",")) fail(`${label}: cell ${key} shows ${on.pay}, Forge gives ${c.payoffs.join(",")}`);
    if (on.eq !== eqCells.has(key)) fail(`${label}: cell ${key} is ${on.eq ? "" : "not "}marked as an equilibrium, against Forge`);
  }
  if (got.equilibria.length !== want.equilibria.length) fail(`${label}: ${got.equilibria.length} equilibria on screen, Forge has ${want.equilibria.length}`);
  want.equilibria.forEach((e, k) => {
    for (const p of want.players) {
      const on = got.equilibria[k] && got.equilibria[k][p];
      if (on !== JSON.stringify(e.players[p].strategies)) fail(`${label}: equilibrium ${k + 1} has ${p} playing ${on} on screen, Forge gives ${JSON.stringify(e.players[p].strategies)}`);
    }
  });
}

// Every game of the open model against Forge; the answer, by game
async function checkAll(label, id, main, scenario) {
  const want = await forge(id, main, scenario);
  if (!Array.isArray(want)) { fail(`${label}: Forge answered ${JSON.stringify(want).slice(0, 200)}`); return {}; }
  const names = await page.$$eval("#view-model [data-game]", (cards) => cards.map((c) => c.dataset.game));
  if (names.join(",") !== want.map((g) => g.game).join(",")) fail(`${label}: the cards are ${names}, Forge's games are ${want.map((g) => g.game)}`);
  for (const g of want) compare(`${label} ${g.game}`, g, await shown(g.game));
  return Object.fromEntries(want.map((g) => [g.game, g]));
}

// The price war in one state of the model; `pure` is the cell the
// equilibrium must be in, as worked out in src/cli/commands/examples
async function check(label, scenario, pure) {
  const games = await checkAll(label, "bagel", "price_war.yaml", scenario);
  if (!games.price_war) return { stable: [], ranking: [{}] };
  const eq = pureCells(games.price_war).join(";");
  if (eq !== pure) fail(`${label}: Forge's pure equilibria are ${eq}, not ${pure}`);
  return games.rivalry;
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

// The Games set: every file, every game
const drawn = (p) => p.waitForFunction(() => [...document.querySelectorAll("#view-model [data-game]")]
  .every((c) => !/Forge is working out/.test(c.textContent)), null, { timeout: 60000 });
const set = (await (await fetch(`${base}/examples.json`)).json()).find((x) => x.id === "games");
if (!set) fail("there is no Games set");
const files = set ? Object.keys(set.files) : [];
let gamesSeen = 0;
for (const file of files) {
  const p = await browser.newPage({ viewport: { width: 390, height: 844 } });
  p.on("pageerror", (e) => fail(`${file}: ${e.message}`));
  page = p;
  await p.goto(`${base}/?example=games`);
  await ready(p);
  if (file !== set.main) {
    await p.click('.tab[data-tab="files"]');
    await p.click(`[data-main="${file}"]`);
    await ready(p);
    await p.click('.tab[data-tab="model"]');
  }
  await p.waitForSelector("#view-model [data-game]");
  await drawn(p);
  const games = await checkAll(file, "games", file, null);
  gamesSeen += Object.keys(games).length;
  if (file === "stag-hunt.yaml") {
    // The mixed equilibrium: stag with (3 - c) / 4, worked out by hand
    const mixed = (g, c) => {
      const e = g.stag_hunt.equilibria.find((x) => x.kind === "mixed");
      if (!e || Math.abs(e.players.me.strategies.stag - (3 - c) / 4) > 1e-12) fail(`stag hunt at c = ${c}: Forge's mixed equilibrium is ${JSON.stringify(e)}, not stag ${(3 - c) / 4}`);
    };
    mixed(games, 0);
    let before = await generation(p);
    await p.click('[data-scn="c3"]');
    await settle(p, before);
    await drawn(p);
    mixed(await checkAll("stag-hunt.yaml (c3)", "games", file, "c3"), 1.5);
    before = await generation(p);
    await p.click('[data-scn=""]');
    await settle(p, before);
    before = await generation(p);
    await p.fill('[data-input="c"]', "1");
    await settle(p, before);
    await drawn(p);
    const shownNow = await shown("stag_hunt");
    const stag = shownNow && shownNow.equilibria.map((e) => e.me && JSON.parse(e.me).stag).find((x) => x > 0 && x < 1);
    if (!(Math.abs(stag - 0.5) < 1e-12)) fail(`typing 1 for c shows the mixed equilibrium with stag ${stag}, not 0.5`);
    // A formula on a card is offered the game's results by name
    await p.click('[data-edit="kinship.stag_share"]');
    const field = p.locator('[data-finput="kinship.stag_share"]');
    await field.fill("=stag_hunt.equilibrium_3.me.st");
    await field.evaluate((el) => el.setSelectionRange(el.value.length, el.value.length));
    await field.dispatchEvent("input");
    const offered = await p.locator('[data-fslot="kinship.stag_share"] [data-suggest] [data-sug]').allTextContents();
    if (!offered.includes("stag_hunt.equilibrium_3.me.stag")) fail(`typing "stag_hunt.equilibrium_3.me.st" on a card offers ${offered.join(", ") || "nothing"}`);
    // The card lists the game's results under those names, with calculate's values
    const listed = await p.$$eval('[data-results="stag_hunt"] [data-scalar]', (rows) => rows.map((r) => r.dataset.scalar));
    if (!listed.includes("stag_hunt.even_mix.me.stag") || !listed.includes("stag_hunt.equilibrium_3.me.stag")) fail(`the stag hunt's card lists ${listed.length} results: ${listed.slice(0, 5)}`);
  }
  await p.close();
}
if (gamesSeen < files.length) fail(`only ${gamesSeen} games in the ${files.length} files of the Games set`);

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Games: the bagel's price war shows Forge's payoffs, equilibria, ranking, stable players and matches in the plan and both scenarios, an input typed on a card moves them, and the Tools tab runs forge_game; the ${gamesSeen} games of the Games set's ${files.length} files are Forge's, and the stag hunt's scenario and a typed cost move its mixed equilibrium.`);
