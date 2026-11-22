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
// - the Tools tab runs forge_game on the open model and shows the whole
//   answer, the tool's JSON, with Copy and Download;
// - in the Games set (examples/games), every game of each of its ten files,
//   opened from the Files tab, is Forge's: each cell, each equilibrium with
//   its chances and its stability, each ranking, stable list and match; the
//   stag hunt's scenario c3 and a cost typed on its card move the mixed
//   equilibrium as Forge does, from 3/4 stag to (3 - c) / 4;
// - a game of five players alike (the public goods game, the volunteer's
//   dilemma) is drawn by count (ADR-073), not as its 32 profiles: each number
//   is every player's payoff in Forge's cells with that strategy and that
//   split of the others, the cells marked are those of Forge's pure
//   equilibria, and the equilibria are listed by split with Forge's count of
//   profiles and payoffs; the public goods game's scenario break_even, where
//   all 32 profiles are equilibria, lists them as 6 splits;
// - the dynamics (ADR-073): the shares and the average payoff after the last
//   generation are Forge's; hawks end at 2/3 in hawk-dove, the stag hunt ends
//   all stag from 0.8 and all hare in its scenario few_stags; pointing at the
//   chart shows a generation's shares from Forge's path;
// - the bagel's price war spreads through the city as Forge's path says (99.1%
//   of the shops at $14 after 24 months), and its street's festival of five
//   shops is drawn by count, all keeping, and all paying when the borough
//   matches;
// - the acquisition's sealed bids (deal_game.yaml) are Forge's, with both
//   bidding 30% and both 40% marked, and only 30% in its scenario;
// - a link names one model of a set (`?example=bagel&model=price_war.yaml`)
//   and opens on its games, and a model the set does not have opens the
//   set's own.
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
    stableEq: [...card.querySelectorAll("[data-equilibrium]")].map((li) => { const s = li.querySelector("[data-stable-eq]"); return s ? s.dataset.stableEq === "1" : null; }),
    counts: [...card.querySelectorAll("[data-count]")].map((td) => ({ key: td.dataset.count, pay: td.dataset.pay, eq: td.dataset.eq === "1" })),
    splits: [...card.querySelectorAll("[data-split]")].map((li) => ({ split: li.dataset.split, profiles: Number(li.dataset.profiles),
      gets: Object.fromEntries([...li.querySelectorAll("[data-gets]")].map((x) => [x.dataset.gets, x.dataset.payoff])) })),
    mixed: [...card.querySelectorAll('[data-equilibrium="mixed"]')].map((li) => ({ chances: li.dataset.chances, payoff: li.dataset.payoff })),
    dynamics: card.querySelector("[data-dynamics]") ? {
      generations: card.querySelector("[data-dynamics]").dataset.generations,
      payoff: card.querySelector("[data-dynamics]").dataset.payoff,
      shares: Object.fromEntries([...card.querySelectorAll("[data-share]")].map((x) => [x.dataset.share, x.dataset.value])),
    } : null,
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
  if (want.dynamics) {
    const last = want.dynamics.path[want.dynamics.path.length - 1];
    if (!got.dynamics) fail(`${label}: Forge has dynamics, the card draws none`);
    else {
      if (got.dynamics.generations !== String(want.dynamics.generations)) fail(`${label}: ${got.dynamics.generations} generations on screen, Forge ran ${want.dynamics.generations}`);
      if (got.dynamics.payoff !== String(last.payoff)) fail(`${label}: the last average payoff on screen is ${got.dynamics.payoff}, Forge's is ${last.payoff}`);
      for (const [s, v] of Object.entries(last.shares)) if (got.dynamics.shares[s] !== String(v)) fail(`${label}: ${s}'s last share is ${got.dynamics.shares[s]} on screen, Forge's is ${v}`);
    }
  } else if (got.dynamics) fail(`${label}: the card draws dynamics Forge has not run`);
  if (got.counts.length) { compareCounts(label, want, got); return; }
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
    if (got.stableEq[k] !== (e.stable === undefined ? null : e.stable)) fail(`${label}: equilibrium ${k + 1} is shown ${got.stableEq[k] === null ? "without a stability" : got.stableEq[k] ? "stable" : "unstable"}, Forge says ${e.stable}`);
  });
}

// A game of players alike, drawn by count, against Forge's cells: the test
// files each cell's payoffs under (own strategy | how many others play each)
function compareCounts(label, want, got) {
  const strategies = want.strategies[want.players[0]];
  const counts = (list) => strategies.map((s) => list.filter((x) => x === s).length);
  const expect = new Map();
  for (const c of want.cells) c.strategies.forEach((own, i) => {
    const key = `${own}|${counts(c.strategies.filter((_, j) => j !== i)).join(",")}`;
    if (!expect.has(key)) expect.set(key, new Set());
    expect.get(key).add(String(c.payoffs[i]));
  });
  if (got.cells.length) fail(`${label}: drawn by count and also as ${got.cells.length} profiles`);
  if (got.counts.length !== expect.size) fail(`${label}: ${got.counts.length} numbers on screen, Forge's cells give ${expect.size}`);
  const marked = new Set();
  const groups = new Map();
  for (const e of want.equilibria.filter((x) => x.kind === "pure")) {
    const all = counts(want.players.map((p) => Object.entries(e.players[p].strategies).find(([, x]) => x === 1)[0]));
    strategies.forEach((s, i) => { if (all[i]) marked.add(`${s}|${all.map((n, j) => n - (j === i ? 1 : 0)).join(",")}`); });
    const g = groups.get(all.join(",")) || { profiles: 0, gets: {} };
    g.profiles += 1;
    for (const p of want.players) {
      const s = Object.entries(e.players[p].strategies).find(([, x]) => x === 1)[0];
      if (g.gets[s] !== undefined && g.gets[s] !== String(e.players[p].payoff)) fail(`${label}: players who play ${s} get different payoffs in one split`);
      g.gets[s] = String(e.players[p].payoff);
    }
    groups.set(all.join(","), g);
  }
  for (const [key, pays] of expect) {
    const on = got.counts.find((x) => x.key === key);
    if (pays.size !== 1) fail(`${label}: Forge's cells give ${key} the payoffs ${[...pays]}, so its players are not alike`);
    if (!on) { fail(`${label}: no number for ${key} on screen`); continue; }
    if (!pays.has(on.pay)) fail(`${label}: ${key} shows ${on.pay}, Forge's cells give ${[...pays]}`);
    if (on.eq !== marked.has(key)) fail(`${label}: ${key} is ${on.eq ? "" : "not "}marked as played in an equilibrium, against Forge`);
  }
  if (got.splits.length !== groups.size) fail(`${label}: ${got.splits.length} splits listed, Forge's pure equilibria make ${groups.size}`);
  for (const [split, g] of groups) {
    const on = got.splits.find((x) => x.split === split);
    if (!on) { fail(`${label}: the split ${split} is not listed`); continue; }
    if (on.profiles !== g.profiles) fail(`${label}: the split ${split} lists ${on.profiles} profiles, Forge has ${g.profiles}`);
    if (JSON.stringify(on.gets) !== JSON.stringify(g.gets)) fail(`${label}: the split ${split} shows ${JSON.stringify(on.gets)}, Forge gives ${JSON.stringify(g.gets)}`);
  }
  const mixed = want.equilibria.filter((x) => x.kind !== "pure");
  if (got.mixed.length !== mixed.length) fail(`${label}: ${got.mixed.length} mixed equilibria on screen, Forge has ${mixed.length}`);
  mixed.forEach((e, k) => {
    const play = e.players[want.players[0]];
    for (const p of want.players) if (JSON.stringify(e.players[p].strategies) !== JSON.stringify(play.strategies)) fail(`${label}: Forge's mixed equilibrium ${k + 1} is not symmetric`);
    if (!got.mixed[k] || got.mixed[k].chances !== JSON.stringify(play.strategies) || got.mixed[k].payoff !== String(play.payoff)) fail(`${label}: mixed equilibrium ${k + 1} shows ${JSON.stringify(got.mixed[k])}, Forge gives ${JSON.stringify(play)}`);
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

// The price war across the city: Forge's path, compared by checkAll above;
// from a tenth of the shops at $14, 99.1% cut after 24 months
{
  const war = (await forge("bagel", "price_war.yaml", null)).find((g) => g.game === "price_war");
  const end = war && war.dynamics.path[war.dynamics.path.length - 1].shares.cut;
  if (!(Math.abs(end - 0.9913363782163622) < 1e-12)) fail(`price war: Forge's cut share after 24 months is ${end}`);
  const shownEnd = (await shown("price_war")).dynamics;
  if (!shownEnd || shownEnd.shares.cut !== String(end)) fail(`price war: the screen ends with ${JSON.stringify(shownEnd)}`);
}

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
await page.waitForFunction(() => /<pre|msg bad/.test(document.querySelector('[data-out="game"]').innerHTML), null, { timeout: 60000 });
const answer = await page.textContent('[data-out="game"] pre');
if (!answer.includes('"game": "price_war"') || !answer.includes('"cells"') || !answer.includes('"dynamics"')) fail(`the Tools tab's forge_game answered: ${answer.slice(0, 200)}`);
// The whole answer, never cut (ADR-077): what is on screen is the tool's JSON
// for the files as they are now (0.55 typed, in the plan)
{
  const direct = await page.evaluate(async () => {
    const { call } = await import("./forge.js");
    const saved = JSON.parse(localStorage.getItem("forge-studio-v3"));
    return call("game", { files: saved.files, main: saved.main, scenario: null });
  });
  let shown = null;
  try { shown = JSON.parse(answer); } catch (e) { fail(`the Tools tab's forge_game answer is not whole JSON (${answer.length} characters, ends ${JSON.stringify(answer.slice(-20))})`); }
  if (shown && JSON.stringify(shown) !== JSON.stringify(direct)) fail("the Tools tab's forge_game answer is not the tool's answer");
  const copy = await page.$('[data-out="game"] [data-copy]'), dl = await page.$('[data-out="game"] [data-dl-answer]');
  if (!copy || !dl) fail("the Tools tab's answer has no Copy or no Download button");
}
await page.close();

// The street's festival (bagel/street_fund.yaml): five shops, drawn by count,
// all keep; the borough's match makes all pay
{
  const p = await browser.newPage({ viewport: { width: 390, height: 844 } });
  p.on("pageerror", (e) => fail(`street_fund.yaml: ${e.message}`));
  page = p;
  await p.goto(`${base}/?example=bagel`);
  await ready(p);
  await p.click('.tab[data-tab="files"]');
  await p.click('[data-main="street_fund.yaml"]');
  await ready(p);
  await p.click('.tab[data-tab="model"]');
  await p.waitForSelector('[data-game="street"] table');
  await p.waitForFunction(() => !/Forge is working out/.test(document.querySelector('[data-game="street"]').textContent), null, { timeout: 60000 });
  await checkAll("street_fund.yaml", "bagel", "street_fund.yaml", null);
  const street = await shown("street");
  if (street.counts.length !== 10 || street.cells.length) fail(`street_fund.yaml: ${street.counts.length} numbers by count and ${street.cells.length} profiles`);
  if (street.splits.map((x) => x.split).join(";") !== "0,5") fail(`street_fund.yaml: the splits listed are ${street.splits.map((x) => x.split)}, not all five keeping`);
  const before = await generation(p);
  await p.click('[data-scn="borough_matches"]');
  await settle(p, before);
  await checkAll("street_fund.yaml (borough_matches)", "bagel", "street_fund.yaml", "borough_matches");
  const matched = await shown("street");
  if (matched.splits.map((x) => x.split).join(";") !== "5,0") fail(`street_fund.yaml (borough_matches): the splits listed are ${matched.splits.map((x) => x.split)}, not all five paying`);
  await p.close();
}

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
  if (file === "stag-hunt.yaml") {
    // The dynamics: from 0.8 every hunter goes for the stag; from 0.7, the
    // scenario few_stags, every hunter hunts hare (by hand and by egttools)
    const p2 = await browser.newPage({ viewport: { width: 390, height: 844 } });
    p2.on("pageerror", (e) => fail(`stag-hunt.yaml (dynamics): ${e.message}`));
    page = p2;
    await p2.goto(`${base}/?example=games`);
    await ready(p2);
    await p2.waitForSelector('[data-dynamics="stag_hunt"]');
    await drawn(p2);
    const all = await checkAll("stag-hunt.yaml (dynamics)", "games", file, null);
    const end = (g) => g.stag_hunt.dynamics.path[g.stag_hunt.dynamics.path.length - 1].shares.stag;
    if (!(end(all) > 1 - 1e-9)) fail(`stag hunt from 0.8: Forge ends with stag ${end(all)}, not all stag`);
    const before = await generation(p2);
    await p2.click('[data-scn="few_stags"]');
    await settle(p2, before);
    await drawn(p2);
    const few = await checkAll("stag-hunt.yaml (few_stags)", "games", file, "few_stags");
    if (end(few) !== 0) fail(`stag hunt from 0.7: Forge ends with stag ${end(few)}, not all hare`);
    const shownShare = (await shown("stag_hunt")).dynamics.shares.stag;
    if (shownShare !== "0") fail(`stag hunt from 0.7: the screen ends with stag ${shownShare}, not 0`);
    await p2.close();
    page = p;
  }
  if (file === "hawk-dove.yaml") {
    // Hawks end at V / C = 2/3, and the chart shows a generation from the path
    const hd = games.hawk_dove;
    const last = hd && hd.dynamics.path[hd.dynamics.path.length - 1].shares.hawk;
    if (!(Math.abs(last - 2 / 3) < 1e-9)) fail(`hawk-dove: Forge ends with hawk ${last}, not 2/3`);
    const chart = p.locator('[data-dynamics="hawk_dove"] .path-chart');
    await chart.scrollIntoViewIfNeeded();
    const box = await chart.boundingBox();
    await p.mouse.move(box.x + box.width * 0.6, box.y + box.height / 2);
    const tip = await p.$eval('[data-dynamics="hawk_dove"] .path-tip', (el) => ({ gen: Number(el.dataset.gen), shares: el.dataset.shares, text: el.textContent }));
    const at = hd && hd.dynamics.path[tip.gen];
    if (!at || !(tip.gen > 0 && tip.gen < hd.dynamics.generations)) fail(`pointing at 60% of hawk-dove's chart shows generation ${tip.gen}`);
    else if (tip.shares !== JSON.stringify(at.shares) || !tip.text.startsWith(`Generation ${tip.gen}: hawk `)) fail(`pointing at hawk-dove's chart shows ${tip.shares} ("${tip.text}"), Forge's generation ${tip.gen} is ${JSON.stringify(at.shares)}`);
  }
  if (file === "public-goods.yaml") {
    // At r = 5 all 32 profiles are equilibria: 6 splits on screen
    const before = await generation(p);
    await p.click('[data-scn="break_even"]');
    await settle(p, before);
    await drawn(p);
    const even = await checkAll("public-goods.yaml (break_even)", "games", file, "break_even");
    const got = await shown("pg");
    if (even.pg && even.pg.equilibria.length !== 32) fail(`public goods at r = 5: Forge has ${even.pg.equilibria.length} equilibria, not 32`);
    if (got.splits.length !== 6) fail(`public goods at r = 5: ${got.splits.length} splits on screen, not 6`);
  }
  if (file === "public-goods.yaml" || file === "volunteers-dilemma.yaml") {
    const name = file === "public-goods.yaml" ? "pg" : "vd";
    const got = await shown(name);
    if (!got || got.cells.length || got.counts.length !== 10) fail(`${file}: ${name} is not drawn by count (${got && got.counts.length} numbers, ${got && got.cells.length} profiles)`);
  }
  await p.close();
}
if (gamesSeen < files.length) fail(`only ${gamesSeen} games in the ${files.length} files of the Games set`);

// The acquisition's sealed bids, opened from the Files tab: two pure
// equilibria and a mixed one, worked out by hand in src/cli/commands/game.rs;
// the scenario leaves both bidding 30%
{
  const p = await browser.newPage({ viewport: { width: 390, height: 844 } });
  p.on("pageerror", (e) => fail(`deal_game.yaml: ${e.message}`));
  page = p;
  await p.goto(`${base}/?example=acquisition`);
  await ready(p);
  await p.click('.tab[data-tab="files"]');
  await p.click('[data-main="deal_game.yaml"]');
  await ready(p);
  await p.click('.tab[data-tab="model"]');
  await p.waitForSelector('[data-game="sealed_bids"] table');
  await drawn(p);
  const bids = await checkAll("deal_game.yaml", "acquisition", "deal_game.yaml", null);
  const pure = bids.sealed_bids ? pureCells(bids.sealed_bids).join(";") : "";
  if (pure !== "bid_30,bid_30;bid_40,bid_40") fail(`deal_game.yaml: the pure equilibria are ${pure}`);
  const marked = await p.$$eval('[data-game="sealed_bids"] [data-eq]', (tds) => tds.map((td) => td.dataset.cell).join(";"));
  if (marked !== "bid_30,bid_30;bid_40,bid_40") fail(`deal_game.yaml: the cells marked are ${marked}`);
  const before = await generation(p);
  await p.click('[data-scn="thornvale_cost_only"]');
  await settle(p, before);
  await drawn(p);
  const only = await checkAll("deal_game.yaml (thornvale_cost_only)", "acquisition", "deal_game.yaml", "thornvale_cost_only");
  const one = only.sealed_bids ? pureCells(only.sealed_bids).join(";") : "";
  if (one !== "bid_30,bid_30") fail(`deal_game.yaml (thornvale_cost_only): the pure equilibria are ${one}`);
  await p.close();
}

// A link opens one model of a set (`?example=bagel&model=price_war.yaml`), so
// a post can link to the price war itself: the game is on the Model view with
// no tap on Files. A model the set does not have opens the set's own.
{
  const p = await browser.newPage({ viewport: { width: 390, height: 844 } });
  p.on("pageerror", (e) => fail(`link to price_war.yaml: ${e.message}`));
  page = p;
  await p.goto(`${base}/?example=bagel&model=price_war.yaml`);
  await ready(p);
  const title = await p.textContent("#model-title");
  if (title !== "price_war.yaml") fail(`a link to price_war.yaml opened ${title}`);
  await p.waitForSelector('[data-game="price_war"] table');
  await p.waitForFunction(() => !/Forge is working out/.test(document.querySelector('[data-game="price_war"]').textContent), null, { timeout: 60000 });
  await checkAll("link to price_war.yaml", "bagel", "price_war.yaml", null);
  await p.goto(`${base}/?example=bagel&model=no_such_file.yaml`);
  await ready(p);
  const fallback = await p.evaluate(() => [document.querySelector("#model-title").textContent, document.querySelectorAll("[data-game]").length]);
  if (fallback[0] !== "Bagel Lune" || fallback[1] !== 0) fail(`a link to a model the set does not have opened ${fallback[0]} with ${fallback[1]} games`);
  await p.close();
}

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Games: a link opens the bagel's price war directly; the price war shows Forge's payoffs, equilibria, ranking, stable players and matches in the plan and both scenarios, an input typed on a card moves them, and the Tools tab runs forge_game and shows its whole answer; the ${gamesSeen} games of the Games set's ${files.length} files are Forge's, the games of five players are drawn by count, the dynamics end where Forge's do (hawks at 2/3, the stag hunt all stag or all hare), and the stag hunt's scenario and a typed cost move its mixed equilibrium.`);
