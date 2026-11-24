// A game edited on its card (ADR-076 decision 5), in a headless browser at
// 390 px, on the inspection game (examples/games) and the bagel's price war and
// its rivalry (price_war.yaml):
//
// - "change this game" holds Forge's own names: the game, each player and
//   each strategy to rename, the strategies and players to delete;
// - a strategy added, a player renamed, the strategy deleted again, and a
//   player added to a tournament each change the card to what `game` answers
//   for the files `edit` wrote, cell by cell and standing by standing;
// - a payoff's formula under its value while the card is open for changes,
//   changed to a formula and to a number, a refused one shown as Forge words it;
// - a change Forge refuses (a strategy of the game a tournament plays) is
//   shown in the card with Forge's own message, and nothing changes.
//
//   node studio/tests/gameedit.mjs <built Studio dir or address>

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
let page, id, main;
async function visit(example, model, game) {
  if (page) await page.close();
  [id, main] = [example, model];
  page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  page.on("pageerror", (e) => fail(e.message));
  await page.goto(`${base}/?example=${id}&model=${main}`);
  await page.waitForFunction(() => document.body.dataset.state !== "loading", null, { timeout: 120000 });
  await page.waitForSelector(`[data-game="${game}"] table`);
}

// Forge's answer for the example's files with these changes made, in order:
// the games, or the message `edit` refuses with
const forge = (changes) => page.evaluate(async ([id, main, changes]) => {
  const { call } = await import("./forge.js");
  const set = (await (await fetch("examples.json")).json()).find((x) => x.id === id);
  try {
    const { files } = await call("edit", { files: { ...set.files }, main, changes });
    return { games: await call("game", { files: { ...set.files, ...files }, main }) };
  } catch (e) { return { why: e.message }; }
}, [id, main, changes]);

// What the cards show: a table's cells, a tournament's players in rank order
const shown = () => page.evaluate(() => Object.fromEntries([...document.querySelectorAll("#view-model [data-game]")].map((card) => [card.dataset.game, {
  cells: [...card.querySelectorAll("[data-cell]")].map((td) => `${td.dataset.cell}=${td.dataset.pay}`),
  ranking: [...card.querySelectorAll("[data-standing]")].map((tr) => `${tr.dataset.standing}=${tr.dataset.score}`),
}])));

const settled = () => page.waitForFunction(() => document.body.dataset.state !== "loading"
  && [...document.querySelectorAll("#view-model [data-game]")].every((c) => !/Forge is working out/.test(c.textContent)), null, { timeout: 60000 });

async function matches(label, changes) {
  const want = await forge(changes);
  if (!want.games) { fail(`${label}: Forge refused: ${want.why}`); return; }
  const got = await shown();
  for (const g of want.games) {
    const cells = g.kind === "tournament" ? [] : g.cells.map((c) => `${c.strategies.join(",")}=${c.payoffs.join(",")}`);
    const ranking = g.kind === "tournament" ? g.ranking.map((s) => `${s.player}=${s.score}`) : [];
    if (!got[g.game]) { fail(`${label}: no card for ${g.game}`); continue; }
    if (JSON.stringify(got[g.game].cells) !== JSON.stringify(cells)) fail(`${label}: ${g.game}'s cells are ${got[g.game].cells}, Forge's are ${cells}`);
    if (JSON.stringify(got[g.game].ranking) !== JSON.stringify(ranking)) fail(`${label}: ${g.game}'s standings are ${got[g.game].ranking}, Forge's are ${ranking}`);
  }
  if (Object.keys(got).length !== want.games.length) fail(`${label}: ${Object.keys(got)} cards for Forge's ${want.games.map((g) => g.game)}`);
}

const editor = (game) => `[data-gedit="${game}"]`;
const open = (game) => page.evaluate((g) => { document.querySelector(`[data-gedit="${g}"]`).open = true; }, game);
const rowOf = (game, row) => `${editor(game)} [data-grow="${row}"]`;

// The inspection game: each player has its own strategies
await visit("games", "inspection.yaml", "inspection");
await open("inspection");
const offered = await page.$$eval(`${rowOf("inspection", "rename")} option`, (os) => os.map((o) => o.value));
const wantOffered = ["inspection", "inspection.players.ai", "inspection.players.human", "inspection.strategies.ai.comply", "inspection.strategies.ai.cheat", "inspection.strategies.human.audit", "inspection.strategies.human.trust"];
if (offered.join(",") !== wantOffered.join(",")) fail(`rename offers ${offered}, Forge's names are ${wantOffered}`);
await matches("as opened", []);

// A strategy for both players, a player renamed, and the strategy deleted
const changes = [];
await page.fill(`${rowOf("inspection", "add_strategy")} input`, "bluff");
await page.click(`${rowOf("inspection", "add_strategy")} button`);
changes.push({ op: "add_strategy", path: "inspection.strategies.bluff" });
await page.waitForSelector('[data-game="inspection"] [data-cell="bluff,bluff"]');
await settled();
await matches("add strategy", changes);

await open("inspection");
await page.selectOption(`${rowOf("inspection", "rename")} select`, "inspection.players.ai");
await page.fill(`${rowOf("inspection", "rename")} input`, "machine");
await page.click(`${rowOf("inspection", "rename")} button`);
changes.push({ op: "rename", path: "inspection.players.ai", new_name: "machine" });
await page.waitForFunction(() => /machine vs human/.test(document.querySelector('[data-game="inspection"]').textContent), null, { timeout: 60000 });
await settled();
await matches("rename player", changes);

await open("inspection");
await page.selectOption(`${rowOf("inspection", "delete_strategy")} select`, "inspection.strategies.bluff");
await page.click(`${rowOf("inspection", "delete_strategy")} button`);
changes.push({ op: "delete_strategy", path: "inspection.strategies.bluff" });
await page.waitForFunction(() => document.querySelectorAll('[data-game="inspection"] [data-cell]').length === 4, null, { timeout: 60000 });
await settled();
await matches("delete strategy", changes);

// The price war and its tournament
await visit("bagel", "price_war.yaml", "price_war");
await open("rivalry");
await page.fill(`${rowOf("rivalry", "add_player")} input`, "newcomer");
await page.selectOption(`${rowOf("rivalry", "add_player")} select`, "random");
await page.click(`${rowOf("rivalry", "add_player")} button`);
const tournament = [{ op: "add_player", path: "rivalry.players.newcomer", plays: "random" }];
await page.waitForSelector('[data-game="rivalry"] [data-standing="newcomer"]');
await settled();
await matches("add tournament player", tournament);

// A change Forge refuses is shown as Forge words it, and nothing moves: the
// tournament plays the price war's strategies
await open("price_war");
const before = JSON.stringify(await shown());
await page.fill(`${rowOf("price_war", "add_strategy")} input`, "bluff");
await page.click(`${rowOf("price_war", "add_strategy")} button`);
await page.waitForSelector(`${editor("price_war")} [data-gmsg]:not([hidden])`);
const refusal = await forge([...tournament, { op: "add_strategy", path: "price_war.strategies.bluff" }]);
const message = await page.textContent(`${editor("price_war")} [data-gmsg]`);
if (!refusal.why) fail("Forge accepts a strategy for the game a tournament plays");
else if (message !== refusal.why) fail(`the card says "${message}", Forge says "${refusal.why}"`);
if (JSON.stringify(await shown()) !== before) fail("a refused change moved the cards");
if (await page.evaluate(() => document.documentElement.scrollWidth > innerWidth)) fail("the cards are wider than the screen");

// A payoff cell: its formula is under its value while the card can be
// changed, and a change to it is Forge's `edit` (a formula, or a number)
await visit("bagel", "price_war.yaml", "price_war");
const cellOf = (path) => `[data-game="price_war"] [data-cellpath="${path}"]`;
const T = "price_war.payoffs.cut.hold";
if (await page.isVisible(cellOf(T))) fail("a payoff's button is shown although the card is not open for changes");
await open("price_war");
if (!(await page.isVisible(cellOf(T)))) fail("a payoff's button is not shown on a card open for changes");
if ((await page.textContent(cellOf(T))) !== "=market.T") fail(`the button for ${T} says ${await page.textContent(cellOf(T))}, the file says =market.T`);
const cellChanges = [];
async function changeCell(path, text) {
  await page.click(cellOf(path));
  await page.fill(`[data-cellform="${path}"] input`, text);
  await page.click(`[data-cellform="${path}"] [data-cellsave]`);
}
await changeCell(T, "=market.T + 1");
cellChanges.push({ op: "set_formula", path: T, formula: "=market.T + 1" });
await page.waitForSelector(`[data-cellform="${T}"]`, { state: "detached", timeout: 60000 });
await settled();
await matches("payoff formula", cellChanges);
if ((await page.textContent(cellOf(T))) !== "=market.T + 1") fail("the card does not show the new formula");

// A number on a payoff that is a formula, and what is not accepted
const P = "price_war.payoffs.cut.cut";
await changeCell(P, "7");
cellChanges.push({ op: "set_value", path: P, value: 7 });
await page.waitForSelector(`[data-cellform="${P}"]`, { state: "detached", timeout: 60000 });
await settled();
await matches("payoff number", cellChanges);

const beforeCells = JSON.stringify(await shown());
await changeCell(T, "=nothing_here + 1");
await page.waitForSelector(`[data-cellform="${T}"] [data-cellmsg]:not([hidden])`, { timeout: 60000 });
const bad = await forge([...cellChanges, { op: "set_formula", path: T, formula: "=nothing_here + 1" }]);
const cellMessage = await page.textContent(`[data-cellform="${T}"] [data-cellmsg]`);
if (!bad.why) fail("Forge accepts a payoff that reads a name the model does not have");
else if (cellMessage !== bad.why) fail(`the card says "${cellMessage}", Forge says "${bad.why}"`);
await page.fill(`[data-cellform="${T}"] input`, "abc");
await page.click(`[data-cellform="${T}"] [data-cellsave]`);
if ((await page.textContent(`[data-cellform="${T}"] [data-cellmsg]`)) !== "A payoff is a number or a formula starting with =.") fail("a payoff that is neither is not said so");
if (JSON.stringify(await shown()) !== beforeCells) fail("a refused payoff moved the cards");
await page.click(`[data-cellform="${T}"] [data-cellcancel]`);
if (!(await page.isVisible(cellOf(T)))) fail("Cancel does not bring the payoff's button back");

// A game's other numbers: the dynamics' generations, a tournament's continuation
// and a noise the file leaves at its default (added), each with its own button
async function setNumber(game, path, text) {
  const button = `[data-game="${game}"] [data-cellpath="${path}"]`;
  await page.click(button);
  await page.fill(`[data-game="${game}"] [data-cellform="${path}"] input`, text);
  await page.click(`[data-game="${game}"] [data-cellform="${path}"] [data-cellsave]`);
}
const generations = () => page.evaluate(() => document.querySelector('[data-dynamics="price_war"]').dataset.generations);
await open("rivalry");
const GEN = "price_war.dynamics.generations";
if (!(await page.isVisible(`[data-game="price_war"] [data-cellpath="${GEN}"]`))) fail("the dynamics' generations have no button on a card open for changes");
await setNumber("price_war", GEN, "30");
cellChanges.push({ op: "set_value", path: GEN, value: 30 });
await page.waitForFunction(() => document.querySelector('[data-dynamics="price_war"]')?.dataset.generations === "30", null, { timeout: 60000 });
await settled();
await matches("dynamics generations", cellChanges);
const want = await forge(cellChanges);
if ((await generations()) !== String(want.games.find((g) => g.game === "price_war").dynamics.generations)) fail("the dynamics run for other generations than Forge's");

const CONT = "rivalry.tournament.continuation";
await setNumber("rivalry", CONT, "0.5");
cellChanges.push({ op: "set_value", path: CONT, value: 0.5 });
await page.waitForFunction(() => /chance 0\.5/.test(document.querySelector('[data-game="rivalry"]').textContent), null, { timeout: 60000 });
await settled();
await matches("continuation", cellChanges);

const NOISE = "rivalry.tournament.noise";
await setNumber("rivalry", NOISE, "0.1");
cellChanges.push({ op: "set_value", path: NOISE, value: 0.1 });
await page.waitForFunction(() => /flipped with chance 0\.1/.test(document.querySelector('[data-game="rivalry"]').textContent), null, { timeout: 60000 });
await settled();
await matches("noise added", cellChanges);

// Shares that do not add up are refused as Forge words it
const START = "price_war.dynamics.start.cut";
await setNumber("price_war", START, "0.9");
await page.waitForSelector(`[data-cellform="${START}"] [data-cellmsg]:not([hidden])`, { timeout: 60000 });
const noStart = await forge([...cellChanges, { op: "set_value", path: START, value: 0.9 }]);
if (!noStart.why) fail("Forge accepts start shares that do not add up");
else if ((await page.textContent(`[data-cellform="${START}"] [data-cellmsg]`)) !== noStart.why) fail("the card does not say what Forge says of start shares that do not add up");

// A tournament's player's habit: a select on its row while the card is open
// for changes; a habit Forge refuses (a chance the new habit does not take) is
// said as Forge words it, and the select goes back
const habitOf = (game, player) => `[data-game="${game}"] [data-habitpath="${game}.players.${player}"]`;
await page.evaluate(() => { document.querySelector('[data-gedit="rivalry"]').open = false; });
if (await page.isVisible(habitOf("rivalry", "steady"))) fail("a habit's select is shown although the card is not open for changes");
await open("rivalry");
await page.selectOption(habitOf("rivalry", "steady"), "random");
cellChanges.push({ op: "set_habit", path: "rivalry.players.steady", plays: "random" });
await page.waitForFunction(() => /random/.test(document.querySelector('[data-game="rivalry"] [data-standing="steady"] .plays')?.textContent ?? ""), null, { timeout: 60000 });
await settled();
await matches("habit", cellChanges);
await visit("games", "axelrod.yaml", "axelrod");
await open("axelrod");
await page.selectOption(habitOf("axelrod", "rand"), "tit_for_tat");
const noHabit = await forge([{ op: "set_habit", path: "axelrod.players.rand", plays: "tit_for_tat" }]);
await page.waitForSelector('[data-game="axelrod"] [data-gmsg]:not([hidden])', { timeout: 60000 });
if (!noHabit.why) fail("Forge accepts a habit that does not take the player's chance");
else if ((await page.textContent('[data-game="axelrod"] [data-gmsg]')) !== noHabit.why) fail("the card does not say what Forge says of a habit that does not take the player's chance");
if ((await page.inputValue(habitOf("axelrod", "rand"))) !== "random") fail("a refused habit stays chosen in the select");

// A game of players alike writes one payoff for each strategy: one button each,
// and a change moves every column of its row, as Forge's cells do
await visit("games", "public-goods.yaml", "pg");
const countOf = (key) => `[data-game="pg"] [data-count="${key}"]`;
const countPay = async (changes, strategy, others) => {
  const g = (await forge(changes)).games.find((x) => x.game === "pg");
  return g.cells.find((c) => c.strategies[0] === strategy && c.strategies.slice(1).every((x) => x === others)).payoffs[0];
};
const pgButton = (strategy) => `[data-game="pg"] [data-cellpath="pg.payoffs.${strategy}"]`;
if (await page.isVisible(pgButton("keep"))) fail("a count form's payoff button is shown although the card is not open for changes");
await open("pg");
if (!(await page.isVisible(pgButton("keep")))) fail("the count form's payoff has no button on a card open for changes");
const pgChanges = [{ op: "set_formula", path: "pg.payoffs.keep", formula: "=endowment + 1" }];
await page.click(pgButton("keep"));
await page.fill('[data-cellform="pg.payoffs.keep"] input', "=endowment + 1");
await page.click('[data-cellform="pg.payoffs.keep"] [data-cellsave]');
await page.waitForFunction(() => document.querySelector('[data-game="pg"] [data-count="keep|4,0"]')?.dataset.pay === "11", null, { timeout: 60000 });
await settled();
if (Number(await page.getAttribute(countOf("keep|4,0"), "data-pay")) !== (await countPay(pgChanges, "keep", "contribute"))) fail("the count form's keep row is not Forge's after its formula was changed");
if (Number(await page.getAttribute(countOf("keep|0,4"), "data-pay")) !== (await countPay(pgChanges, "keep", "keep"))) fail("the count form's keep row is not the same number in each column");
pgChanges.push({ op: "set_value", path: "pg.payoffs.contribute", value: 3 });
await open("pg");
await page.click(pgButton("contribute"));
await page.fill('[data-cellform="pg.payoffs.contribute"] input', "3");
await page.click('[data-cellform="pg.payoffs.contribute"] [data-cellsave]');
await page.waitForFunction(() => document.querySelector('[data-game="pg"] [data-count="contribute|4,0"]')?.dataset.pay === "3", null, { timeout: 60000 });
await settled();
if (Number(await page.getAttribute(countOf("contribute|0,4"), "data-pay")) !== (await countPay(pgChanges, "contribute", "keep"))) fail("the count form's contribute row is not Forge's after a number was set");
await open("pg");
await page.click(pgButton("keep"));
await page.fill('[data-cellform="pg.payoffs.keep"] input', "=nothing_here");
await page.click('[data-cellform="pg.payoffs.keep"] [data-cellsave]');
await page.waitForSelector('[data-cellform="pg.payoffs.keep"] [data-cellmsg]:not([hidden])', { timeout: 60000 });
const noPay = await forge([...pgChanges, { op: "set_formula", path: "pg.payoffs.keep", formula: "=nothing_here" }]);
if (!noPay.why) fail("Forge accepts a count form's payoff that reads a name the model does not have");
else if ((await page.textContent('[data-cellform="pg.payoffs.keep"] [data-cellmsg]')) !== noPay.why) fail("the card does not say what Forge says of the count form's payoff");

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log("Game edit: a strategy added and deleted, a player renamed, a tournament's player added and a payoff, a player's habit, a count form's payoffs, a game's dynamics and a tournament's length and noise changed (a formula, a number) are Forge's games for the files `edit` wrote, and a refused change shows Forge's message.");
