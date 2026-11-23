// Forge Studio's game cards (ADR-068, ADR-073): forge game's answer, drawn.
// Every number here is in the answer of the `game` tool; the page only lays
// them out. A cell's payoffs and an equilibrium's chances are also kept in
// data- attributes, in full, for the screen test (tests/games.mjs).

import { esc } from "./text.js";
import { fmt } from "./format.js";

const pct = (p) => fmt(p, "", "%");
// The pure strategy a player plays in an equilibrium, or null in a mixed one
const pureOf = (play) => { const on = Object.entries(play.strategies).find(([, p]) => p === 1); return on ? on[0] : null; };

// A game of more than two players whose players are alike: each has the same
// strategies, and what one gets depends only on its own strategy and on how
// many of the others play each one. Read off Forge's cells, compared as
// they are: every player's payoff, under its own strategy and the others'
// counts, is the same number. Then the game is drawn by count (ADR-073), one
// row a strategy and one column a way the others split, and not as a list of
// every profile (32 for five players).
function byCount(r) {
  const [first, ...rest] = r.players;
  if (r.players.length < 3) return null;
  const strategies = r.strategies[first];
  if (rest.some((p) => r.strategies[p].join("\u0001") !== strategies.join("\u0001"))) return null;
  const pay = new Map();
  const counts = (list) => strategies.map((s) => list.filter((x) => x === s).length);
  for (const c of r.cells) {
    for (let i = 0; i < r.players.length; i++) {
      const key = `${c.strategies[i]}|${counts(c.strategies.filter((_, j) => j !== i)).join(",")}`;
      if (pay.has(key) && pay.get(key) !== c.payoffs[i]) return null;
      pay.set(key, c.payoffs[i]);
    }
  }
  const splits = [...new Set([...pay.keys()].map((k) => k.split("|")[1]))]
    .map((k) => k.split(",").map(Number))
    .sort((a, b) => { for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return a[i] - b[i]; return 0; });
  return { strategies, splits, pay, counts };
}
const splitName = (strategies, split) => split.map((n, i) => `${n} ${strategies[i]}`).join(", ");

export function tableCard(r) {
  const key = (list) => list.join("\u0001");
  const pure = new Set(r.equilibria.filter((e) => e.kind === "pure").map((e) => key(r.players.map((p) => pureOf(e.players[p])))));
  const cellHtml = (c) => {
    const eq = pure.has(key(c.strategies));
    return `<td class="${eq ? "eq" : ""}" data-cell="${esc(c.strategies.join(","))}" data-pay="${c.payoffs.join(",")}"${eq ? ' data-eq="1"' : ""}>${c.payoffs.map((v) => `<span class="pay">${esc(fmt(v))}</span>`).join("")}${eq ? '<span class="eq-mark">equilibrium</span>' : ""}</td>`;
  };
  const alike = byCount(r);
  let h = `<div class="game-head"><b>${esc(r.game)}</b><span class="note">${alike ? `${r.players.length} players alike (${esc(r.players.join(", "))})` : esc(r.players.join(" vs "))}, played once</span></div>`;
  if (r.players.length === 2) {
    const [a, b] = r.players, cols = r.strategies[b];
    h += `<div class="scroll-x"><table class="report payoffs"><thead><tr><th scope="col">${esc(a)} ╲ ${esc(b)}</th>${cols.map((c) => `<th scope="col">${esc(c)}</th>`).join("")}</tr></thead><tbody>` +
      r.strategies[a].map((x) => `<tr><th scope="row">${esc(x)}</th>${cols.map((y) => cellHtml(r.cells.find((c) => c.strategies[0] === x && c.strategies[1] === y))).join("")}</tr>`).join("") +
      `</tbody></table></div><p class="note">Each cell: what ${esc(a)} gets, and below it what ${esc(b)} gets.</p>`;
  } else if (alike) {
    h += countTable(r, alike);
  } else {
    h += `<div class="scroll-x"><table class="report payoffs"><thead><tr>${r.players.map((p) => `<th scope="col">${esc(p)}</th>`).join("")}<th scope="col">payoffs</th></tr></thead><tbody>` +
      r.cells.map((c) => `<tr>${c.strategies.map((x) => `<th scope="row">${esc(x)}</th>`).join("")}${cellHtml(c)}</tr>`).join("") +
      `</tbody></table></div><p class="note">Payoffs in the order of the players.</p>`;
  }
  h += alike ? countEquilibria(r, alike) : equilibriaList(r, r.equilibria);
  if (!r.mixed_searched && !r.symmetric_searched) h += `<p class="note">With more than two players Forge lists the pure equilibria only.</p>`;
  if (r.symmetric_searched) h += `<p class="note">Forge lists the pure equilibria and those in which every player mixes the same way, not those in which players mix differently.</p>`;
  if (r.ties.length) h += `<p class="note">A tie makes the game degenerate: equilibria between those listed may be missing (${esc(r.ties.join("; "))}).</p>`;
  if (r.dominated.length) h += `<p class="note" data-dominated>Strictly dominated, removed in this order: ${r.dominated.map((d) => `${esc(d.player)}'s ${esc(d.strategy)} (by ${esc(d.by)})`).join(", ")}.</p>`;
  if (r.dynamics) h += dynamicsView(r);
  return h;
}

// One player's payoff by its own strategy (rows) and by how the others split
// (columns); a cell is marked when a player in it plays a pure equilibrium
function countTable(r, { strategies, splits, pay, counts }) {
  const others = r.players.length - 1;
  const marked = new Set();
  for (const e of r.equilibria.filter((x) => x.kind === "pure")) {
    const all = counts(r.players.map((p) => pureOf(e.players[p])));
    strategies.forEach((s, i) => { if (all[i] > 0) marked.add(`${s}|${all.map((n, j) => n - (j === i ? 1 : 0)).join(",")}`); });
  }
  const two = strategies.length === 2;
  const head = (split) => two ? String(split[0]) : splitName(strategies, split);
  return `<div class="scroll-x"><table class="report payoffs counts"><thead><tr><th scope="col">own ╲ others</th>${splits.map((x) => `<th scope="col">${esc(head(x))}</th>`).join("")}</tr></thead><tbody>` +
    strategies.map((s) => `<tr><th scope="row">${esc(s)}</th>${splits.map((x) => {
      const k = `${s}|${x.join(",")}`, eq = marked.has(k);
      return `<td class="${eq ? "eq" : ""}" data-count="${esc(k)}" data-pay="${pay.get(k)}"${eq ? ' data-eq="1"' : ""}><span class="pay">${esc(fmt(pay.get(k)))}</span>${eq ? '<span class="eq-mark">equilibrium</span>' : ""}</td>`;
    }).join("")}</tr>`).join("") +
    `</tbody></table></div><p class="note">What one player gets, by its own strategy (row) and ${two ? `by how many of the other ${others} play ${esc(strategies[0])} (column)` : `by how the other ${others} split (column)`}. Every player gets the same in its place; the ${r.cells.length} profiles are these ${strategies.length * splits.length} numbers.</p>`;
}

// The equilibria of a game of players alike: the pure ones by how many play
// each strategy (the 32 of the public goods game at r = 5 are 6 splits),
// then each symmetric mixed one
function countEquilibria(r, { strategies, counts }) {
  const groups = new Map(), mixed = [];
  for (const e of r.equilibria) {
    if (e.kind !== "pure") { mixed.push(e); continue; }
    const split = counts(r.players.map((p) => pureOf(e.players[p]))).join(",");
    if (!groups.has(split)) groups.set(split, []);
    groups.get(split).push(e);
  }
  const n = r.equilibria.length;
  let h = `<p class="sub" style="margin-top:12px">${n} equilibri${n === 1 ? "um" : "a"}</p><ol class="eqs">`;
  for (const [split, list] of groups) {
    const nums = split.split(",").map(Number);
    const gets = strategies.map((s, i) => {
      if (!nums[i]) return null;
      const play = Object.values(list[0].players).find((p) => p.strategies[s] === 1);
      return `<span data-gets="${esc(s)}" data-payoff="${play.payoff}">a player who plays ${esc(s)} gets ${esc(fmt(play.payoff))}</span>`;
    }).filter(Boolean).join("; ");
    h += `<li data-split="${esc(split)}" data-profiles="${list.length}">pure: ${esc(splitName(strategies, nums))}, ${list.length === 1 ? "one profile" : `${list.length} profiles, one for each way to choose who`} (${gets})</li>`;
  }
  h += mixed.map((e) => {
    const play = e.players[r.players[0]];
    return `<li data-equilibrium="mixed" data-chances="${esc(JSON.stringify(play.strategies))}" data-payoff="${play.payoff}">mixed: every player ${esc(Object.entries(play.strategies).filter(([, x]) => x > 0).map(([s, x]) => `${s} ${pct(x)}`).join(", "))} (gets ${esc(fmt(play.payoff))})${stability(e)}</li>`;
  }).join("") + `</ol>`;
  return h;
}
const stability = (e) => e.stable === undefined ? "" : `<span class="stable" data-stable-eq="${e.stable ? 1 : 0}">${e.stable ? "stable: a population returns to it" : "unstable: a population moves away"}</span>`;

function equilibriaList(r, list) {
  const n = list.length;
  return `<p class="sub" style="margin-top:12px">${n} equilibri${n === 1 ? "um" : "a"}</p><ol class="eqs">` + list.map((e, k) => `<li data-equilibrium="${k + 1}">${esc(e.kind)}: ${r.players.map((p) => {
    const play = e.players[p], one = pureOf(play);
    const how = one !== null ? one : Object.entries(play.strategies).filter(([, x]) => x > 0).map(([s, x]) => `${s} ${pct(x)}`).join(", ");
    return `<span data-play="${esc(p)}" data-chances="${esc(JSON.stringify(play.strategies))}"><b>${esc(p)}</b> ${esc(how)} (gets ${esc(fmt(play.payoff))})</span>`;
  }).join("; ")}${stability(e)}</li>`).join("") + `</ol>`;
}

// ---------- the dynamics (ADR-073): the shares, generation by generation ----------
// The strategies' colours: the first three slots of the categorical palette
// (studio.css, --series-1 to --series-3), in the strategies' order; a fourth
// and later strategy wears the muted ink, named in the legend
const SERIES = ["var(--series-1)", "var(--series-2)", "var(--series-3)"];
const W = 320, H = 150, L = 34, R = 8, T = 8, B = 22;
const paths = new Map();
// At most this many points a line: a path of 100,000 generations is drawn
// from every k-th, its last always included
const POINTS = 400;
function sample(n, most) {
  const step = Math.max(1, Math.ceil(n / most));
  const at = [];
  for (let i = 0; i < n; i += step) at.push(i);
  if (at[at.length - 1] !== n - 1) at.push(n - 1);
  return at;
}
function dynamicsView(r) {
  const d = r.dynamics, strategies = r.strategies[r.players[0]];
  paths.set(r.game, { d, strategies });
  const last = d.path[d.path.length - 1];
  const x = (g) => L + (W - L - R) * g / d.generations, y = (s) => T + (H - T - B) * (1 - s);
  const lines = strategies.map((s, i) => `<polyline fill="none" stroke="${SERIES[i] || "var(--muted)"}" stroke-width="2" stroke-linejoin="round" stroke-linecap="round" points="${sample(d.path.length, POINTS).map((g) => `${x(g).toFixed(2)},${y(d.path[g].shares[s]).toFixed(2)}`).join(" ")}"/>`).join("");
  const grid = [0, 0.5, 1].map((s) => `<line x1="${L}" x2="${W - R}" y1="${y(s)}" y2="${y(s)}" class="grid"/><text x="${L - 4}" y="${y(s) + 3.5}" text-anchor="end">${pct(s)}</text>`).join("");
  const axis = `<text x="${L}" y="${H - 6}" text-anchor="start">0</text><text x="${W - R}" y="${H - 6}" text-anchor="end">${esc(fmt(d.generations))}</text><text x="${(L + W - R) / 2}" y="${H - 6}" text-anchor="middle">generation</text>`;
  const legend = strategies.map((s, i) => `<span class="key" data-share="${esc(s)}" data-value="${last.shares[s]}"><i style="background:${SERIES[i] || "var(--muted)"}"></i>${esc(s)} <b>${esc(pct(last.shares[s]))}</b></span>`).join("");
  const rows = sample(d.path.length, 21).map((g) => `<tr data-gen="${g}"><th scope="row">${g}</th>${strategies.map((s) => `<td>${esc(pct(d.path[g].shares[s]))}</td>`).join("")}<td>${esc(fmt(d.path[g].payoff))}</td></tr>`).join("");
  return `<div class="dynamics" data-dynamics="${esc(r.game)}" data-generations="${d.generations}" data-payoff="${last.payoff}">
    <p class="sub" style="margin-top:12px">the shares, generation by generation <span data-help-slot="dynamics"></span></p>
    <p class="note">A population in which each meets strangers, started from ${esc(strategies.map((s) => `${s} ${pct(d.path[0].shares[s])}`).join(", "))}, with the baseline ${esc(fmt(d.baseline))} added to every payoff. After ${esc(fmt(d.generations))} generations:</p>
    <div class="legend">${legend}<span class="key">average payoff <b>${esc(fmt(last.payoff))}</b></span></div>
    <svg class="path-chart" viewBox="0 0 ${W} ${H}" role="img" aria-label="The share of each strategy over ${esc(fmt(d.generations))} generations">${grid}${axis}${lines}<line class="cross" x1="0" x2="0" y1="${T}" y2="${H - B}" visibility="hidden"/><rect class="hit" x="${L}" y="${T}" width="${W - L - R}" height="${H - T - B}" fill="transparent"/></svg>
    <p class="note path-tip" aria-live="polite">Touch or point at the chart for a generation's shares.</p>
    <details class="results"><summary>the path as a table</summary><div class="scroll-x"><table class="report"><thead><tr><th scope="col">generation</th>${strategies.map((s) => `<th scope="col">${esc(s)}</th>`).join("")}<th scope="col">average payoff</th></tr></thead><tbody>${rows}</tbody></table></div></details>
  </div>`;
}
// The crosshair and the generation under the pointer
export function pathHover(e) {
  const svg = e.target.closest && e.target.closest(".path-chart");
  const box = svg && svg.closest("[data-dynamics]");
  const p = box && paths.get(box.dataset.dynamics);
  if (!p) return;
  const rect = svg.getBoundingClientRect();
  const at = (e.clientX - rect.left) / rect.width * W;
  const g = Math.round(Math.min(Math.max((at - L) / (W - L - R), 0), 1) * p.d.generations);
  const cross = svg.querySelector(".cross"), cx = L + (W - L - R) * g / p.d.generations;
  cross.setAttribute("x1", cx); cross.setAttribute("x2", cx); cross.setAttribute("visibility", "visible");
  const step = p.d.path[g];
  const tip = box.querySelector(".path-tip");
  tip.dataset.gen = g;
  tip.dataset.shares = JSON.stringify(step.shares);
  tip.textContent = `Generation ${g}: ${p.strategies.map((s) => `${s} ${pct(step.shares[s])}`).join(", ")}; average payoff ${fmt(step.payoff)}.`;
}

export function tournamentCard(r) {
  const length = r.rounds != null ? `${r.rounds} rounds a match` : `goes on after each round with chance ${fmt(r.continuation)}`;
  let h = `<div class="game-head"><b>${esc(r.game)}</b><span class="note">${esc(r.round)} played again and again: ${esc(length)}${r.noise ? `, a move flipped with chance ${esc(fmt(r.noise))}` : ""}${r.self_play ? ", each also against itself" : ""}</span><span data-help-slot="tournament"></span></div>`;
  h += `<div class="scroll-x"><table class="report ranking"><thead><tr><th scope="col">player</th><th scope="col">rank</th><th scope="col">a round</th><th scope="col">cooperates</th></tr></thead><tbody>` +
    r.ranking.map((s) => `<tr data-standing="${esc(s.player)}" data-score="${s.score}"><th scope="row">${esc(s.player)}<span class="plays">${esc(s.plays.replace(/_/g, " "))}</span></th><td>${s.rank}</td><td>${esc(fmt(s.score))}</td><td>${esc(pct(s.cooperation))}</td></tr>`).join("") +
    `</tbody></table></div>`;
  h += `<p class="note" data-stable>Collectively stable (nobody does better against it than it does against itself): ${r.stable.length ? esc(r.stable.join(", ")) : "none"}.</p>`;
  // In the ranking's order (the object's keys come back sorted)
  const names = r.ranking.map((s) => s.player);
  h += `<p class="sub" style="margin-top:12px">a round, row against column</p><div class="scroll-x"><table class="report"><thead><tr><th scope="col"></th>${names.map((x) => `<th scope="col">${esc(x)}</th>`).join("")}</tr></thead><tbody>` +
    names.map((a) => `<tr><th scope="row">${esc(a)}</th>${names.map((b) => `<td data-match="${esc(a)},${esc(b)}" data-pay="${r.matches[a][b]}">${esc(fmt(r.matches[a][b]))}</td>`).join("")}</tr>`).join("") + `</tbody></table></div>`;
  return h;
}
