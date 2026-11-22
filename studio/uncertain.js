// An uncertain model on Forge Studio's cards (ADR-076): the cards show the
// plan (Forge's `calculate` with `plan: true`), each output the model asks
// about gets its range from `forge simulate` run with the model's own runs,
// seed and sampling, and an input that is a distribution is a card of its
// own, with the kinds and parameter names of Forge's function list. Every
// number is Forge's; the page lays them out. A number carries data-num (a
// JSON pointer into simulate's answer) and data-value, for the screen test.

import { esc } from "./text.js";

// The pause after a change before the range is worked out again (a
// simulation takes about half a second; a calculation waits 120 ms)
export const BAND_PAUSE = 400;

// The uncertain inputs Forge calculated at their plan figures, by name
export const uncertainCount = (calc) => (calc && calc.at_plan ? Object.keys(calc.at_plan).length : 0);
export const uncertainText = (n) => `uncertain: ${n} input${n === 1 ? " is a range" : "s are ranges"}`;

// The note at the top of the Model view of an uncertain model
export function uncertainNote(n, band) {
  const failed = band && band.error ? `<br>The ranges could not be worked out: ${esc(band.error)}` : "";
  return `<b>${esc(uncertainText(n))}.</b> Each card shows the plan, and each result the simulation asks about has its range under it, from <span class="mono">forge simulate</span> with the model's runs and seed. <a href="#an-sim" data-show="analyze">Analyze</a> runs other outputs and seeds.${failed}`;
}

// Forge's reason for not weighing a model's scenarios, said in the page's
// words first when none has a probability (ADR-076)
export function scenarioWhy(message) {
  const text = String(message).replace(/^Validation error: /, "");
  const lead = /No scenario has a probability/.test(text)
    ? `<p class="note">These scenarios have no probabilities: they are choices to compare, not chances. Tap one on the Model tab to apply it.</p>` : "";
  return `${lead}<p class="note">${esc(text)}</p>`;
}

// ---------- the distribution card ----------
// The arguments of a call as written, split at the commas outside any (), {}
// or "text"
function splitArgs(text) {
  const out = [];
  let depth = 0, quoted = false, from = 0;
  for (let i = 0; i < text.length; i++) {
    const ch = text[i];
    if (ch === '"') quoted = !quoted;
    else if (!quoted && "({[".includes(ch)) depth++;
    else if (!quoted && ")}]".includes(ch)) depth--;
    else if (!quoted && depth === 0 && ch === ",") { out.push(text.slice(from, i).trim()); from = i + 1; }
  }
  out.push(text.slice(from).trim());
  return out.filter((x, i) => x !== "" || i < out.length - 1);
}
// A formula that is a distribution: its kind as written and its arguments
export function distributionOf(formula) {
  const m = /^=\s*(MC\.[A-Za-z]+)\s*\(([\s\S]*)\)\s*$/.exec(formula || "");
  return m ? { kind: m[1], args: splitArgs(m[2]) } : null;
}
// Forge's distributions, from its function list: each one's name and the
// names of its parameters, from its syntax (=MC.PERT(min, mode, max))
export function distributionsOf(functions) {
  return (functions || []).filter((f) => /^MC\./.test(f.name)).map((f) => {
    const m = /\(([^)]*)\)/.exec(f.syntax || "");
    return { name: f.name, syntax: f.syntax, params: m ? m[1].split(",").map((p) => p.trim()).filter(Boolean) : [] };
  });
}
const same = (a, b) => a.toLowerCase() === b.toLowerCase();

// The card of an input that is a distribution: its plan (filled in by the
// page as for a result), its kind and one field per parameter
export function distCard(s, d, dists, { name, unit, deleteButton, editButton }) {
  const known = dists.find((x) => same(x.name, d.kind));
  const params = known ? known.params : d.args.map((_, i) => `argument ${i + 1}`);
  const id = (i) => `dp-${s.path.replace(/[^A-Za-z0-9_-]/g, "_")}-${i}`;
  return `<div class="row dist" data-card="${esc(s.path)}" data-path="${esc(s.path)}"${s.path.startsWith("@") ? "" : ` data-scalar="${esc(s.path)}"`} data-dist="${esc(s.path)}">
    <div class="res-top"><div class="res-name">${esc(name)}${unit}</div><span class="tag">a range</span>${deleteButton}</div>
    <div class="dist-plan"><span class="lab">plan</span><span class="res-val" data-val></span><span class="note" data-dist-mean></span></div>
    <div class="chips" role="group" aria-label="Distribution of ${esc(name)}">${dists.map((x) => `<button type="button" class="chip" data-dkind="${esc(x.name)}" aria-pressed="${same(x.name, d.kind)}">${esc(x.name.slice(3))}</button>`).join("")}</div>
    <div class="dist-params">${params.map((p, i) => `<div><label class="lab" for="${id(i)}">${esc(p)}</label><input class="txt" id="${id(i)}" data-dparam="${i}" inputmode="decimal" autocomplete="off" value="${esc(d.args[i] ?? "")}"></div>`).join("")}</div>
    <div class="formula-row" data-fslot="${esc(s.path)}"><div class="formula">${esc(s.formula)}</div>${editButton}</div>
    <div class="msg bad" data-dmsg hidden></div></div>`;
}
// The formula a distribution card holds now: its kind and its fields, or
// null while a field is empty
export function cardFormula(card) {
  const kind = card.querySelector('[data-dkind][aria-pressed="true"]').dataset.dkind;
  const args = [...card.querySelectorAll("[data-dparam]")].map((f) => f.value.trim());
  if (args.some((a) => a === "")) return null;
  return `=${kind}(${args.join(", ")})`;
}
// Another kind chosen: its parameters' fields, keeping the values of the
// parameters both kinds have by name
export function chooseKind(card, kind, dists) {
  const now = new Map([...card.querySelectorAll("[data-dparam]")].map((f) => [f.closest("div").querySelector("label").textContent, f.value]));
  const known = dists.find((x) => x.name === kind);
  for (const chip of card.querySelectorAll("[data-dkind]")) chip.setAttribute("aria-pressed", String(chip.dataset.dkind === kind));
  const base = card.dataset.dist.replace(/[^A-Za-z0-9_-]/g, "_");
  card.querySelector(".dist-params").innerHTML = known.params.map((p, i) => `<div><label class="lab" for="dp-${base}-${i}">${esc(p)}</label><input class="txt" id="dp-${base}-${i}" data-dparam="${i}" inputmode="decimal" autocomplete="off" value="${esc(now.get(p) ?? "")}"></div>`).join("");
}

// A kind chosen or a parameter changed on a distribution card: the card's
// formula written by `write` (path, formula), which answers why not, if not
export function listenDistributions(view, dists, write) {
  const keep = async (card) => {
    const formula = cardFormula(card), msg = card.querySelector("[data-dmsg]");
    if (formula === null) return;
    const why = await write(card.dataset.dist, formula);
    if (why) { msg.hidden = false; msg.textContent = why; }
  };
  view.addEventListener("click", (e) => {
    const chip = e.target.closest("[data-dkind]");
    if (chip) { chooseKind(chip.closest("[data-dist]"), chip.dataset.dkind, dists()); keep(chip.closest("[data-dist]")); }
  });
  view.addEventListener("change", (e) => { if (e.target.dataset.dparam) keep(e.target.closest("[data-dist]")); });
}

// ---------- the bands ----------
// The ranges worked out again BAND_PAUSE after the last calculation: the
// previous ones stay, dimmed (body[data-band="running"]), and an answer that
// a newer change has overtaken (S.generation) is dropped. `simulate` runs
// Forge with the model's own runs, seed and sampling; `redraw` shows S.band.
export function bandScheduler(S, simulate, redraw) {
  let timer = 0;
  return () => {
    clearTimeout(timer);
    if (!uncertainCount(S.calc) || !S.structure.analyses.includes("monte_carlo")) { S.band = null; document.body.dataset.band = "none"; return; }
    document.body.dataset.band = "running";
    const generation = S.generation;
    timer = setTimeout(async () => {
      const band = await simulate().then((answer) => ({ answer }), (e) => ({ answer: null, error: e.message }));
      if (generation !== S.generation) return;
      S.band = band;
      document.body.dataset.band = band.error ? "error" : "ready";
      redraw();
    }, BAND_PAUSE);
  };
}
const pointer = (...parts) => "/" + parts.map((p) => String(p).replace(/~/g, "~0").replace(/\//g, "~1")).join("/");
const num = (ptr, v, text) => `<b data-num="${esc(ptr)}" data-value="${esc(String(v))}">${esc(text)}</b>`;

// One output's range: P5 to P95 as a bar, marks at the mean and at the plan,
// and one line per threshold
function bandHtml(name, o, short) {
  const at = (...p) => pointer("monte_carlo_results", "outputs", name, ...p);
  const p = o.percentiles, plan = o.at_plan;
  const lo = Math.min(p.p5, plan ?? p.p5, o.mean), hi = Math.max(p.p95, plan ?? p.p95, o.mean);
  const pos = (v) => ((v - lo) / (hi - lo || 1)) * 100;
  const mark = (cls, v) => `<div class="${cls}" style="left:${pos(v).toFixed(2)}%"></div>`;
  return `<div class="band" data-band="${esc(name)}">
    <div class="band-track" aria-hidden="true"><div class="band-bar" style="left:${pos(p.p5).toFixed(2)}%;width:${Math.max(0.5, pos(p.p95) - pos(p.p5)).toFixed(2)}%"></div>${mark("band-mean", o.mean)}${plan == null ? "" : mark("band-plan", plan)}</div>
    <div class="band-ends"><span>P5 ${num(at("percentiles", "p5"), p.p5, short(p.p5, name))}</span><span>mean ${num(at("mean"), o.mean, short(o.mean, name))}</span><span>P95 ${num(at("percentiles", "p95"), p.p95, short(p.p95, name))}</span></div>
    ${Object.entries(o.thresholds || {}).map(([k, v]) => `<div class="band-th">${esc(k)}: ${num(at("thresholds", k), v, `${(v * 100).toFixed(1)}%`)}</div>`).join("")}</div>`;
}
// The rows of a column the simulation asks about, as a table of ranges
function rowsHtml(column, rows, short) {
  const at = (name, ...p) => pointer("monte_carlo_results", "outputs", name, ...p);
  const ths = [...new Set(rows.flatMap(([, o]) => Object.keys(o.thresholds || {})))];
  return `<details class="results band-rows" data-band="${esc(column)}"><summary>the range of each row (${rows.length})</summary><div class="scroll-x"><table class="report"><thead><tr><th scope="col">row</th><th scope="col">P5</th><th scope="col">mean</th><th scope="col">P95</th>${ths.map((t) => `<th scope="col">${esc(t)}</th>`).join("")}</tr></thead><tbody>` +
    rows.map(([name, o]) => `<tr><th scope="row">${esc(/\(row (\d+)\)$/.exec(name)[1])}</th><td>${num(at(name, "percentiles", "p5"), o.percentiles.p5, short(o.percentiles.p5, name))}</td><td>${num(at(name, "mean"), o.mean, short(o.mean, name))}</td><td>${num(at(name, "percentiles", "p95"), o.percentiles.p95, short(o.percentiles.p95, name))}</td>${ths.map((t) => `<td>${t in (o.thresholds || {}) ? num(at(name, "thresholds", t), o.thresholds[t], `${(o.thresholds[t] * 100).toFixed(1)}%`) : ""}</td>`).join("")}</tr>`).join("") +
    `</tbody></table></div></details>`;
}
// Each output's range under its card, and each distribution's mean, from
// simulate's answer; `open` keeps a row table open across redraws
export function drawBands(root, answer, short, open) {
  for (const old of root.querySelectorAll("[data-band]")) old.remove();
  if (!answer) return;
  const r = answer.monte_carlo_results;
  const order = r.output_order || Object.keys(r.outputs);
  const columns = new Map();
  for (const name of order) {
    const m = /^(.*) \(row \d+\)$/.exec(name);
    if (m) { if (!columns.has(m[1])) columns.set(m[1], []); columns.get(m[1]).push([name, r.outputs[name]]); continue; }
    const card = root.querySelector(`[data-card="${CSS.escape(name)}"] [data-val]`);
    if (card) card.insertAdjacentHTML("afterend", bandHtml(name, r.outputs[name], short));
  }
  for (const [column, rows] of columns) {
    const card = root.querySelector(`[data-card="${CSS.escape(column)}"]`);
    if (!card) continue;
    card.insertAdjacentHTML("beforeend", rowsHtml(column, rows, short));
    if (open.has("band:" + column)) card.querySelector("details[data-band]").open = true;
  }
  for (const card of root.querySelectorAll("[data-dist]")) {
    const input = r.inputs[card.dataset.dist], el = card.querySelector("[data-dist-mean]");
    el.innerHTML = input && input.mean != null ? `mean of the runs ${num(pointer("monte_carlo_results", "inputs", card.dataset.dist, "mean"), input.mean, short(input.mean, card.dataset.dist))}` : "";
  }
  const note = r.iterations.toLocaleString("en-US") + " runs" + (r.seed != null ? `, seed ${r.seed}` : "") + `, ${r.sampling.replace("_", " ")}`;
  for (const band of root.querySelectorAll(".band")) band.insertAdjacentHTML("beforeend", `<div class="band-note">The dark line is the plan, the green one the mean. ${esc(note)}.</div>`);
}
