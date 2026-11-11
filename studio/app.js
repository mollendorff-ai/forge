// Forge Studio (ADR-056): the YAML is the model, the cards show it, and every
// number on screen comes from Forge through forge-wasm (studio/forge.js).
// This file holds the page: state, views and events. It reads no YAML and
// calculates nothing; see studio/tests/no-engine.mjs.

import { call, start } from "./forge.js";
import { esc, setInputText, highlightYaml } from "./text.js";
import { fmt, short, spark, sliderRange, readNumber, isPercentName } from "./format.js";

const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const STORE = "forge-studio-v3";

const S = {
  sets: [], set: null, files: {}, main: "", title: "", version: "",
  structure: null, scenarios: null, report: null, scenario: null, calc: null,
  error: null, generation: 0, open: new Set(), prev: {}, yamlFile: "", tab: "model",
  analyze: {}, tools: {},
};

// ---------- names ----------
// A scalar's path is group.name, or name at the top level, or @ns.group.name
// in an included file.
function split(path) {
  let ns = null, rest = path;
  const m = /^@([^.]+)\.(.*)$/.exec(path);
  if (m) [ns, rest] = [m[1], m[2]];
  const dot = rest.lastIndexOf(".");
  return { ns, group: dot < 0 ? null : rest.slice(0, dot), name: dot < 0 ? rest : rest.slice(dot + 1) };
}
const leaf = (path) => split(path).name;
const fileOfNs = (ns) => (ns ? (S.structure.includes.find((i) => i.as === ns) || {}).file : S.main);
const allScalars = () => S.structure ? [...S.structure.scalars, ...S.structure.includes.flatMap((i) => i.scalars)] : [];
const inputs = () => (S.structure ? S.structure.scalars.filter((s) => s.formula == null && s.value != null) : []);
const results = () => (S.structure ? S.structure.scalars.filter((s) => s.formula != null) : []);
const valueOf = (path) => (S.calc && path in S.calc.scalars ? S.calc.scalars[path] : undefined);

// ---------- storage (this browser only) ----------
function save() {
  try { localStorage.setItem(STORE, JSON.stringify({ set: S.set, files: S.files, main: S.main, title: S.title })); } catch (e) { /* private mode */ }
}
function restore() {
  try {
    const saved = JSON.parse(localStorage.getItem(STORE) || "null");
    if (saved && saved.files && saved.main in saved.files) { Object.assign(S, saved); return true; }
  } catch (e) { /* nothing saved */ }
  return false;
}
function openSet(id) {
  const set = S.sets.find((x) => x.id === id) || S.sets[0];
  S.set = set.id; S.files = { ...set.files }; S.main = set.main; S.title = set.title;
  forget();
}
function forget() {
  S.scenario = null; S.open = new Set(); S.prev = {}; S.yamlFile = S.main; S.analyze = {}; S.tools = {};
}

// ---------- Forge ----------
const request = (extra = {}) => ({ files: S.files, main: S.main, ...extra });

async function load() {
  const generation = ++S.generation;
  document.body.dataset.state = "loading";
  S.error = null; S.structure = null; S.calc = null; S.scenarios = null; S.report = null;
  try {
    const structure = await call("model", request());
    if (generation !== S.generation) return;
    S.structure = structure;
    if (structure.scenarios.length) S.scenarios = await call("scenarios", request()).catch(() => null);
    S.report = await call("report", request()).catch(() => null);
    if (S.scenario && !structure.scenarios.includes(S.scenario)) S.scenario = null;
  } catch (e) {
    if (generation !== S.generation) return;
    S.error = e.message;
  }
  if (generation !== S.generation) return;
  renderAll();
  await recalc();
}

async function recalc() {
  if (!S.structure) { finish(); return; }
  const generation = ++S.generation;
  try {
    const calc = await call("calculate", request({ scenario: S.scenario }));
    if (generation !== S.generation) return;
    S.calc = calc; S.error = null;
  } catch (e) {
    if (generation !== S.generation) return;
    S.calc = null; S.error = e.message;
  }
  updateValues();
  finish();
}
function finish() {
  updateStatus();
  document.body.dataset.state = S.error ? "error" : "ready";
  document.body.dataset.generation = S.generation;
}

// ---------- header ----------
function problems() {
  const list = [];
  if (S.error) list.push(S.error);
  if (S.calc) S.calc.unit_warnings.forEach((w) => list.push(w));
  return list;
}
function updateStatus() {
  $("#model-title").textContent = S.title || S.main;
  const inc = S.structure ? S.structure.includes.map((i) => i.file) : [];
  $("#model-files").textContent = [S.main, ...inc].join(" + ");
  const p = problems();
  $("#status").classList.toggle("has-errors", !!S.error);
  $("#status-text").textContent = S.error ? "Problem" : p.length ? `${p.length} warning${p.length > 1 ? "s" : ""}` : S.version ? `Forge ${S.version}` : "Forge";
}
function openSheet(title, html) {
  $("#sheet-title").textContent = title;
  $("#sheet-body").innerHTML = html;
  $("#sheet").hidden = false;
  $("#sheet-close").focus();
}
$("#status").addEventListener("click", () => {
  const p = problems();
  openSheet(p.length ? "Problems" : "Forge",
    p.length ? `<ul>${p.map((x) => `<li>${esc(x)}</li>`).join("")}</ul>`
      : `<p class="note">Forge ${esc(S.version)} calculated this model in your browser. Nothing left this device.</p>`);
});
$("#sheet-close").addEventListener("click", () => { $("#sheet").hidden = true; });
$("#sheet").addEventListener("click", (e) => { if (e.target.id === "sheet") $("#sheet").hidden = true; });
$("#title-btn").addEventListener("click", () => show("files"));

// ---------- Model view ----------
function groupsOf(scalars) {
  const groups = new Map();
  for (const s of scalars) {
    const g = split(s.path).group ?? "";
    if (!groups.has(g)) groups.set(g, []);
    groups.get(g).push(s);
  }
  return groups;
}
const unitOf = (s) => (s.unit ? ` <span class="unit">${esc(s.unit)}</span>` : "");

function scalarCards(list) {
  let h = "";
  for (const s of list.filter((x) => x.formula == null)) {
    const r = sliderRange(s.value, leaf(s.path));
    const id = "in-" + s.path.replace(/[^A-Za-z0-9_-]/g, "_");
    h += `<div class="row" data-card="${esc(s.path)}" data-path="${esc(s.path)}"${s.path.startsWith("@") ? "" : ` data-scalar="${esc(s.path)}"`}>
      <div class="inp-head"><label class="inp-name" for="${id}">${esc(leaf(s.path))}${unitOf(s)}</label><span class="inp-hint" data-hint></span>
      <input class="field" id="${id}" data-input="${esc(s.path)}" inputmode="decimal" autocomplete="off" value="${s.value ?? ""}"></div>
      <input type="range" data-range="${esc(s.path)}" min="${r.min}" max="${r.max}" step="${r.step}" value="${s.value ?? r.min}" aria-label="${esc(leaf(s.path))}">
      ${s.better ? `<div class="note">better ${esc(s.better)}</div>` : ""}${s.notes ? `<div class="note">${esc(s.notes)}</div>` : ""}
      <div class="by-scn" data-byscn hidden></div><div class="err-msg" data-err hidden></div></div>`;
  }
  for (const s of list.filter((x) => x.formula != null)) {
    const calculated = !s.path.startsWith("@");
    h += `<div class="row" data-card="${esc(s.path)}" data-path="${esc(s.path)}"${calculated ? ` data-scalar="${esc(s.path)}"` : ""}><div class="res-name">${esc(leaf(s.path))}${unitOf(s)}</div>
      <div class="res-val" data-val>${calculated ? "" : '<span class="note">used by the model</span>'}</div><div class="formula">${esc(s.formula)}</div></div>`;
  }
  return h;
}
function tagOf(list) {
  const i = list.some((s) => s.formula == null), f = list.some((s) => s.formula != null);
  return i && f ? "inputs and results" : i ? "inputs" : "results";
}
function tableCards(tables, inc = "") {
  let h = "";
  for (const t of tables) {
    const rows = Math.max(0, ...t.columns.map((c) => c.rows || 0));
    h += `<div class="eyebrow"><h2>${esc(t.name.replace(/^@[^.]+\./, ""))}</h2><span class="tag">table${rows ? ` · ${rows} rows` : ""}</span>${inc}</div><div class="card stack">`;
    for (const c of t.columns) {
      const key = `${t.name}.${c.name}`;
      const open = S.open.has(key);
      h += `<div class="row" style="padding:0" data-card="${esc(key)}" data-table="${esc(t.name)}" data-column="${esc(c.name)}"><button type="button" class="col" data-col="${esc(key)}" aria-expanded="${open}">
        <span class="col-name">${esc(c.name)}${c.unit ? ` <span class="unit">${esc(c.unit)}</span>` : ""}</span><span data-spark></span>
        ${c.formula ? `<span class="formula">${esc(c.formula)}</span>` : ""}<span class="col-meta" data-meta></span></button>
        <div class="vals" data-vals ${open ? "" : "hidden"}></div></div>`;
    }
    h += `</div>`;
  }
  return h;
}
function reportCards() {
  if (!S.report || !S.report.sections) return "";
  return S.report.sections.filter((s) => s.rows.length >= 2).map((s) =>
    `<div class="eyebrow"><h2>side by side</h2><span class="tag">forge report · swipe sideways</span></div>
    <div class="card scroll-x"><table class="report"><thead><tr><th scope="col">group</th>${s.keys.map((k) => `<th scope="col">${esc(k)}</th>`).join("")}</tr></thead><tbody>` +
    s.rows.map((r) => `<tr><th scope="row">${esc(r.group)}</th>${s.keys.map((k) => { const v = r.values[k]; return `<td class="${typeof v === "number" && v < 0 ? "neg" : ""}">${esc(fmt(v, k))}</td>`; }).join("")}</tr>`).join("") +
    `</tbody></table></div>`).join("");
}

function renderModel() {
  const v = $("#view-model");
  if (!S.structure) {
    v.innerHTML = `<div class="msg bad">${esc(S.error || "Forge is starting…")}</div><p class="note">Fix it in the YAML tab, or open another model from Files.</p>`;
    return;
  }
  let h = `<div class="msg bad" id="model-error" hidden></div>`;
  if (S.structure.scenarios.length) {
    const info = (n) => (S.scenarios ? S.scenarios.scenarios.find((s) => s.name === n) : null);
    h += `<div class="eyebrow"><h2>scenarios</h2><span class="tag">tap to apply</span></div>
      <div class="chips" role="group" aria-label="Scenarios"><button type="button" class="chip" data-scn="" aria-pressed="${S.scenario === null}">plan</button>${S.structure.scenarios.map((n) => {
        const p = info(n) && info(n).probability;
        return `<button type="button" class="chip" data-scn="${esc(n)}" aria-pressed="${n === S.scenario}">${esc(n)}${p != null ? `<small>${Math.round(p * 100)}%</small>` : ""}</button>`;
      }).join("")}</div><p class="scn-desc" id="scn-desc"></p>`;
  }
  h += reportCards();
  for (const [g, list] of groupsOf(S.structure.scalars)) {
    h += `<div class="eyebrow"><h2>${esc(g || "top level")}</h2><span class="tag">${tagOf(list)}</span></div><div class="card stack">${scalarCards(list)}</div>`;
  }
  h += tableCards(S.structure.tables);
  for (const inc of S.structure.includes) {
    const badge = `<span class="inc">@${esc(inc.as)} · ${esc(inc.file)}</span>`;
    for (const [g, list] of groupsOf(inc.scalars)) {
      h += `<div class="eyebrow"><h2>${esc(g || "top level")}</h2><span class="tag">${tagOf(list)}</span>${badge}</div><div class="card stack">${scalarCards(list)}</div>`;
    }
    h += tableCards(inc.tables, badge);
  }
  v.innerHTML = h;
  S.prev = {};
}

function rowLabels(table) {
  const t = S.calc && S.calc.tables[table];
  if (!t) return [];
  const n = Math.max(0, ...Object.values(t.values).map((x) => x.length));
  const dates = Object.values(t.values).find((col) => col.length === n && col.every((x) => typeof x === "string" && /^\d{4}-\d{2}-\d{2}$/.test(x)));
  return Array.from({ length: n }, (_, i) => (dates ? dates[i] : String(i + 1)));
}
function pulse(card, key, sig) {
  if (!S.calc) return;
  if (S.prev[key] !== undefined && S.prev[key] !== sig) { card.classList.remove("pulse"); void card.offsetWidth; card.classList.add("pulse"); }
  S.prev[key] = sig;
}

function updateValues() {
  if (!S.structure) { renderModel(); return; }
  const problem = $("#model-error");
  if (problem) {
    problem.hidden = !S.error;
    problem.textContent = !S.error ? "" : /MC\.\* distributions/.test(S.error)
      ? "This model is uncertain: some inputs are distributions, so it has no single answer. Run it on Analyze, under Simulate."
      : S.error;
  }
  const d = $("#scn-desc");
  if (d) {
    const info = S.scenarios && S.scenarios.scenarios.find((s) => s.name === S.scenario);
    d.textContent = S.scenario === null ? "The plan as written in the file." : info && info.description ? info.description : "";
  }
  const byPath = new Map(allScalars().map((s) => [s.path, s]));
  for (const card of $$("#view-model [data-path]")) {
    const path = card.dataset.path, s = byPath.get(path), v = valueOf(path);
    // The number Forge answered, in full, for the screen test
    if (card.dataset.scalar) card.dataset.value = v === undefined || v === null ? "" : String(v);
    pulse(card, path, String(v));
    if (s.formula == null) {
      const f = $("[data-input]", card), r = $("[data-range]", card), b = $("[data-byscn]", card);
      const forced = S.scenario !== null && v !== undefined && v !== s.value;
      const shown = v === undefined ? s.value : v;
      if (document.activeElement !== f) f.value = shown ?? "";
      if (document.activeElement !== r && shown != null) r.value = shown;
      f.disabled = r.disabled = forced;
      b.hidden = !forced;
      if (forced) b.textContent = `Set by the ${S.scenario} scenario. The plan has ${fmt(s.value, leaf(path))}.`;
      $("[data-hint]", card).textContent = isPercentName(leaf(path)) && shown != null ? fmt(shown, leaf(path)) : "";
    } else {
      const el = $("[data-val]", card);
      el.textContent = S.calc ? fmt(v, leaf(path)) : "Not calculated";
      el.classList.toggle("neg", typeof v === "number" && v < 0);
    }
  }
  for (const card of $$("#view-model [data-table]")) {
    const t = S.calc && S.calc.tables[card.dataset.table];
    const vals = t ? t.values[card.dataset.column] || [] : [];
    const name = card.dataset.column, meta = $("[data-meta]", card), sp = $("[data-spark]", card), list = $("[data-vals]", card);
    pulse(card, card.dataset.card, JSON.stringify(vals));
    const numbers = vals.length && vals.every((x) => typeof x === "number");
    sp.innerHTML = numbers ? spark(vals) : "";
    meta.innerHTML = !vals.length ? "<span>Not calculated</span>"
      : numbers ? `<span>${esc(short(Math.min(...vals), name))} to ${esc(short(Math.max(...vals), name))}</span><span>last ${esc(fmt(vals[vals.length - 1], name))}</span>`
      : `<span>${esc(vals.slice(0, 3).map((x) => fmt(x, name)).join(", "))}${vals.length > 3 ? ", …" : ""}</span><span>${vals.length} rows</span>`;
    if (!list.hidden) {
      const labels = rowLabels(card.dataset.table);
      list.innerHTML = vals.map((x, i) => `<span><i>${esc(labels[i] ?? i + 1)}</i>${esc(fmt(x, name))}</span>`).join("");
    }
  }
}

let typing = 0;
function setInput(path, value) {
  const { ns, group, name } = split(path);
  const file = fileOfNs(ns);
  const text = setInputText(S.files[file], group, name, value);
  if (text == null) {
    openSheet("Not changed", `<p>The line for <span class="mono">${esc(path)}</span> was not found in ${esc(file)}. Change it on the YAML tab.</p>`);
    return;
  }
  S.files[file] = text;
  const s = allScalars().find((x) => x.path === path);
  if (s) s.value = value;
  save();
  clearTimeout(typing);
  typing = setTimeout(recalc, 120);
}
$("#view-model").addEventListener("input", (e) => {
  const t = e.target, path = t.dataset.input || t.dataset.range;
  if (!path) return;
  const n = readNumber(t.value);
  if (n === null) return;
  const card = t.closest("[data-card]");
  if (t.dataset.range) $("[data-input]", card).value = n; else $("[data-range]", card).value = n;
  setInput(path, n);
});
$("#view-model").addEventListener("click", (e) => {
  const chip = e.target.closest("[data-scn]");
  if (chip) {
    S.scenario = chip.dataset.scn || null;
    $$("[data-scn]").forEach((b) => b.setAttribute("aria-pressed", (b.dataset.scn || null) === S.scenario));
    recalc();
    return;
  }
  const col = e.target.closest("[data-col]");
  if (col) {
    const k = col.dataset.col;
    if (S.open.has(k)) S.open.delete(k); else S.open.add(k);
    col.setAttribute("aria-expanded", S.open.has(k));
    col.parentElement.querySelector("[data-vals]").hidden = !S.open.has(k);
    updateValues();
  }
});

// ---------- YAML view ----------
function renderYaml() {
  const v = $("#view-yaml");
  if (!(S.yamlFile in S.files)) S.yamlFile = S.main;
  v.innerHTML = `
    <p class="lead">The YAML is the model. Every card comes from these files, and a change on a card is written back here, comments and all.</p>
    <div class="chips" role="group" aria-label="File">${Object.keys(S.files).map((n) => `<button type="button" class="chip" data-file="${esc(n)}" aria-pressed="${n === S.yamlFile}">${esc(n)}${n === S.main ? "<small>main</small>" : ""}</button>`).join("")}</div>
    <div class="editor-wrap"><pre class="editor hl" id="yaml-hl" aria-hidden="true"></pre><textarea class="editor ta" id="yaml-editor" wrap="off" spellcheck="false" autocapitalize="off" autocomplete="off" autocorrect="off" aria-label="YAML of ${esc(S.yamlFile)}"></textarea></div>
    <p class="note legend"><span class="t-fn">functions</span> <span class="t-name">names</span> <span class="t-inc">@included values</span> <span class="t-num">numbers</span> <span class="t-op">operators</span> <span class="t-str">"text"</span> · formula colors follow Forge's VS Code extension</p>
    <div class="msg" id="yaml-msg">In use by the model.</div>
    <div class="btns"><button type="button" class="btn primary" id="yaml-apply" disabled>Check and apply</button><button type="button" class="btn" id="yaml-revert" disabled>Undo edits</button></div>`;
  $("#yaml-editor").value = S.files[S.yamlFile];
  paintYaml();
}
function paintYaml() {
  const ta = $("#yaml-editor"), pre = $("#yaml-hl");
  if (!ta || !pre) return;
  pre.innerHTML = highlightYaml(ta.value);
  pre.scrollTop = ta.scrollTop; pre.scrollLeft = ta.scrollLeft;
}
function yamlDirty() {
  const dirty = $("#yaml-editor").value !== S.files[S.yamlFile];
  $("#yaml-apply").disabled = $("#yaml-revert").disabled = !dirty;
  if (dirty) { $("#yaml-msg").className = "msg"; $("#yaml-msg").textContent = "Edited. Check and apply to let Forge read it."; }
  return dirty;
}
const vy = $("#view-yaml");
vy.addEventListener("input", (e) => { if (e.target.id === "yaml-editor") { paintYaml(); yamlDirty(); } });
vy.addEventListener("scroll", (e) => { if (e.target.id === "yaml-editor") paintYaml(); }, true);
vy.addEventListener("keydown", (e) => {
  if (e.target.id === "yaml-editor" && e.key === "Tab") { e.preventDefault(); document.execCommand("insertText", false, "  "); }
});
vy.addEventListener("click", async (e) => {
  const f = e.target.closest("[data-file]");
  if (f) { S.yamlFile = f.dataset.file; renderYaml(); return; }
  if (e.target.id === "yaml-revert") { renderYaml(); return; }
  if (e.target.id === "yaml-apply") {
    const text = $("#yaml-editor").value, msg = $("#yaml-msg");
    const files = { ...S.files, [S.yamlFile]: text };
    try {
      await call("validate", { files, main: S.main });
      S.files = files; save();
      msg.className = "msg"; msg.textContent = "Forge read it. The cards are up to date.";
      $("#yaml-apply").disabled = $("#yaml-revert").disabled = true;
      await load();
    } catch (err) {
      msg.className = "msg bad"; msg.textContent = `Forge did not accept it: ${err.message}`;
    }
  }
});

// ---------- Analyze view ----------
const opt = (path, sel) => `<option value="${esc(path)}" ${path === sel ? "selected" : ""}>${esc(path)}</option>`;
function defaults() {
  const A = S.analyze, outs = results().map((s) => s.path), ins = inputs().map((s) => s.path);
  if (!outs.includes(A.out)) A.out = outs.find((p) => /profit|total|value|net/.test(p)) || outs[0];
  if (!ins.includes(A.vary)) A.vary = ins[0];
  const others = Object.keys(S.files).filter((f) => f !== S.main && !S.structure.includes.some((i) => i.file === f));
  if (!others.includes(A.actual)) A.actual = others.find((f) => /actual/i.test(f)) || others[0];
  A.others = others;
  return A;
}
function renderAnalyze() {
  const v = $("#view-analyze");
  if (!S.structure) { v.innerHTML = `<div class="msg bad">${esc(S.error || "No model")}</div>`; return; }
  const A = defaults(), outs = results(), ins = inputs();
  v.innerHTML = `<div class="jump"><a href="#an-seek">Goal seek</a><a href="#an-scn">Scenarios</a><a href="#an-tor">Tornado</a><a href="#an-sim">Simulate</a><a href="#an-var">Budget against actual</a></div>
  <div class="eyebrow" id="an-seek"><h2>goal seek</h2><span class="tag">forge goal-seek · break-even</span></div>
  <div class="card row form">
    ${outs.length && ins.length ? `<div><label class="lab" for="sk-out">Result</label><select class="sel" id="sk-out">${outs.map((s) => opt(s.path, A.out)).join("")}</select></div>
    <div class="grid2"><div><label class="lab" for="sk-target">Target</label><input class="txt" id="sk-target" inputmode="decimal" value="${esc(A.target ?? "")}" placeholder="0 for break-even"></div>
    <div><label class="lab">Now</label><div class="now" id="sk-now"></div></div></div>
    <div><label class="lab" for="sk-vary">By changing</label><select class="sel" id="sk-vary">${ins.map((s) => opt(s.path, A.vary)).join("")}</select></div>
    <div class="btns"><button class="btn primary" type="button" id="sk-run">Find the value</button><button class="btn" type="button" id="be-run">Break-even</button></div>
    <div id="sk-result"></div>` : `<p class="note">Goal seek needs an input and a result in the model's scalars.</p>`}
  </div>
  <div class="eyebrow" id="an-scn"><h2>scenarios</h2><span class="tag">forge scenarios</span></div>
  <div class="card row" id="scn-box">${S.scenarios ? "" : `<p class="note">This model has no scenarios section.</p>`}</div>
  <div class="eyebrow" id="an-tor"><h2>tornado</h2><span class="tag">forge tornado</span></div>
  <div class="card row"><div class="btns"><button class="btn" type="button" id="tor-run">Rank the inputs</button></div><div id="tor-box"></div></div>
  <div class="eyebrow" id="an-sim"><h2>simulate</h2><span class="tag">forge simulate · Latin hypercube</span></div>
  <div class="card row form">
    <div class="grid2"><div><label class="lab" for="sim-n">Runs</label><select class="sel" id="sim-n">${[1000, 5000, 10000].map((n) => `<option ${n === (A.runs || 5000) ? "selected" : ""}>${n}</option>`).join("")}</select></div>
    <div><label class="lab" for="sim-seed">Seed</label><input class="txt" id="sim-seed" inputmode="numeric" value="${A.seed ?? 7}"></div></div>
    <div class="btns"><button class="btn primary" type="button" id="sim-run">Run the simulation</button></div><div id="sim-box"></div>
  </div>
  <div class="eyebrow" id="an-var"><h2>budget against actual</h2><span class="tag">forge variance</span></div>
  <div class="card row form">${A.others.length ? `<div class="grid2"><div><label class="lab" for="var-file">Actual</label><select class="sel" id="var-file">${A.others.map((f) => `<option ${f === A.actual ? "selected" : ""}>${esc(f)}</option>`).join("")}</select></div>
    <div><label class="lab" for="var-th">Alert above (%)</label><input class="txt" id="var-th" inputmode="decimal" value="${A.threshold ?? 10}"></div></div>
    <div class="btns"><button class="btn primary" type="button" id="var-run">Compare with ${esc(S.main)}</button></div><div id="var-box"></div>`
    : `<p class="note">Add a second model with the same names (the actuals) on the Files tab to compare it with this one.</p>`}</div>`;
  updateSeekNow();
  drawScenarios();
}
function updateSeekNow() {
  const el = $("#sk-now");
  if (el) el.textContent = fmt(valueOf(S.analyze.out), leaf(S.analyze.out || ""));
}
function drawScenarios() {
  const box = $("#scn-box");
  if (!box || !S.scenarios) return;
  const out = S.analyze.out, name = leaf(out || "");
  const rows = S.scenarios.scenarios.map((s) => `<tr><th scope="row">${esc(s.name)}</th><td>${s.probability != null ? Math.round(s.probability * 100) + "%" : "—"}</td><td class="${s.outputs[out] < 0 ? "neg" : ""}">${esc(fmt(s.outputs[out], name))}</td></tr>`).join("");
  const ev = S.scenarios.expected_values ? S.scenarios.expected_values[out] : undefined;
  box.innerHTML = `<div class="scroll-x"><table class="report"><thead><tr><th scope="col">scenario</th><th scope="col">chance</th><th scope="col">${esc(out)}</th></tr></thead><tbody>${rows}</tbody>
    ${ev !== undefined ? `<tfoot><tr><th scope="row">expected</th><td></td><td>${esc(fmt(ev, name))}</td></tr></tfoot>` : ""}</table></div>
    <p class="note">The result chosen under goal seek, in each scenario, and weighted by the chances.</p>`;
}
const fail = (box, e) => { box.innerHTML = `<div class="msg bad">${esc(e.message)}</div>`; };
const busy = (box, text) => { box.innerHTML = `<p class="note">${esc(text)}</p>`; };

async function runSeek(breakEven) {
  const A = S.analyze, box = $("#sk-result");
  const target = breakEven ? 0 : readNumber($("#sk-target").value);
  if (target === null) { box.innerHTML = `<div class="msg bad">Type the value the result should reach.</div>`; return; }
  A.target = target;
  busy(box, "Forge is searching…");
  try {
    const r = breakEven ? await call("break_even", request({ output: A.out, vary: A.vary }))
      : await call("goal_seek", request({ target: A.out, value: target, vary: A.vary }));
    A.solution = r.solution;
    box.innerHTML = `<div class="answer"><div class="note">${esc(r.vary)}</div><div class="res-val">${esc(fmt(Number(r.solution.toFixed(6)), leaf(r.vary)))}</div>
      <div class="note">gives ${esc(r.target)} = ${esc(fmt(r.achieved, leaf(r.target)))}, against ${esc(fmt(valueOf(r.vary), leaf(r.vary)))} now. ${r.converged ? `Found in ${r.iterations} calculations.` : "Forge did not converge."}</div></div>
      <div class="btns"><button class="btn" type="button" id="sk-use">Use this value</button></div>`;
  } catch (e) { fail(box, e); }
}
function drawTornado(t) {
  const box = $("#tor-box"), name = leaf(t.output);
  const lo = Math.min(...t.bars.map((b) => Math.min(b.output_at_low, b.output_at_high)), t.base_value);
  const hi = Math.max(...t.bars.map((b) => Math.max(b.output_at_low, b.output_at_high)), t.base_value);
  const pos = (v) => ((v - lo) / (hi - lo || 1)) * 100;
  box.innerHTML = `<p class="note" style="margin:10px 0">How far ${esc(t.output)} moves when each input goes from its low to its high value, one at a time. The line is the plan, ${esc(fmt(t.base_value, name))}. Copper means the result drops.</p><div class="tor">` +
    t.bars.map((b) => {
      const a = Math.min(b.output_at_low, b.output_at_high), z = Math.max(b.output_at_low, b.output_at_high);
      const down = b.output_at_high < b.output_at_low;
      return `<div class="tor-row"><div class="tor-name"><span>${esc(b.input_name)}</span><span>swing ${esc(short(b.abs_swing, name))}</span></div>
        <div class="tor-track"><div class="tor-bar" style="left:${pos(a)}%;width:${Math.max(0.5, pos(z) - pos(a))}%;background:${down ? "var(--ember)" : "var(--accent)"}"></div><div class="tor-axis" style="left:${pos(t.base_value)}%"></div></div>
        <div class="tor-ends"><span>${esc(fmt(b.input_low, leaf(b.input_name)))} → ${esc(short(b.output_at_low, name))}</span><span>${esc(fmt(b.input_high, leaf(b.input_name)))} → ${esc(short(b.output_at_high, name))}</span></div></div>`;
    }).join("") + "</div>";
}
function drawSim() {
  const A = S.analyze, box = $("#sim-box"), r = A.sim;
  if (!r) return;
  const outs = Object.keys(r.outputs);
  if (!outs.includes(A.simOut)) A.simOut = outs.find((o) => !/\(row \d+\)/.test(o)) || outs[0];
  const o = r.outputs[A.simOut], name = leaf(A.simOut.replace(/ \(row \d+\)$/, ""));
  const th = Object.entries(o.thresholds || {});
  const p = o.percentiles;
  box.innerHTML = `<div style="margin-top:10px"><label class="lab" for="sim-out">Output</label><select class="sel" id="sim-out">${outs.map((x) => opt(x, A.simOut)).join("")}</select></div>
    <div class="stats" style="margin-top:10px">${[["mean", o.mean], ["P5", p.p5], ["P50", p.p50], ["P95", p.p95]].map(([k, v]) => `<div class="stat"><b>${k}</b><span>${esc(short(v, name))}</span></div>`).join("")}</div>
    ${th.map(([k, v]) => `<p class="note" style="margin-top:8px">Chance that ${esc(A.simOut)} ${esc(k)}: <b>${(v * 100).toFixed(1)}%</b></p>`).join("")}
    ${o.drivers && o.drivers.length ? `<p class="sub" style="margin-top:12px">what drives it</p>${o.drivers.map((d) => `<div class="var-nums"><span class="mono">${esc(d.input)}</span><span>${d.rank_correlation.toFixed(2)}</span></div>`).join("")}` : ""}
    <p class="note" style="margin-top:8px">${r.iterations.toLocaleString("en-US")} runs, seed ${esc(A.seed)}. The same seed gives the same numbers as <span class="mono">forge simulate</span>.</p>`;
}
function drawVariance(r) {
  const box = $("#var-box");
  box.innerHTML = `<div class="stats var-sum" style="margin:10px 0">${[["alerts", r.alert_count], ["favorable", r.favorable_count], ["unfavorable", r.unfavorable_count], ["on target", r.on_target_count]].map(([k, v]) => `<div class="stat"><b>${k}</b><span>${v}</span></div>`).join("")}</div>
    <div class="stack">${r.results.map((x) => {
      const pill = x.is_favorable === true ? '<span class="pill good">favorable</span>' : x.is_favorable === false ? '<span class="pill bad">unfavorable</span>' : '<span class="pill none">no direction</span>';
      return `<div class="var ${x.exceeds_threshold ? "" : "dim"}" style="padding:8px 0;border-top:1px solid var(--line)"><div class="var-top"><span class="var-name">${esc(x.name)}</span>${pill}</div>
        <div class="var-nums"><span>${esc(fmt(x.budget, leaf(x.name)))} → ${esc(fmt(x.actual, leaf(x.name)))}</span><span>${x.variance_pct == null ? "" : (x.variance_pct > 0 ? "+" : "") + x.variance_pct.toFixed(1) + "%"}</span></div></div>`;
    }).join("")}</div>`;
}
const va = $("#view-analyze");
va.addEventListener("change", (e) => {
  const A = S.analyze;
  if (e.target.id === "sk-out") { A.out = e.target.value; updateSeekNow(); drawScenarios(); }
  if (e.target.id === "sk-vary") A.vary = e.target.value;
  if (e.target.id === "sim-out") { A.simOut = e.target.value; drawSim(); }
  if (e.target.id === "var-file") A.actual = e.target.value;
});
va.addEventListener("click", async (e) => {
  const A = S.analyze, id = e.target.id;
  if (id === "sk-run" || id === "be-run") return runSeek(id === "be-run");
  if (id === "sk-use") {
    setInput(A.vary, Number(A.solution.toFixed(6)));
    $("#sk-result").insertAdjacentHTML("beforeend", `<p class="note">Written to ${esc(fileOfNs(split(A.vary).ns))}.</p>`);
    return;
  }
  if (id === "tor-run") {
    const box = $("#tor-box");
    busy(box, "Forge is moving each input…");
    try { drawTornado(await call("tornado", request())); } catch (err) { fail(box, err); }
  }
  if (id === "sim-run") {
    const box = $("#sim-box");
    A.runs = Number($("#sim-n").value);
    A.seed = readNumber($("#sim-seed").value) ?? 7;
    busy(box, `Forge is running ${A.runs.toLocaleString("en-US")} simulations…`);
    try {
      const r = await call("simulate", request({ iterations: A.runs, seed: A.seed, scenario: S.scenario }));
      A.sim = r.monte_carlo_results; drawSim();
    } catch (err) { fail(box, err); }
  }
  if (id === "var-run") {
    const box = $("#var-box");
    A.threshold = readNumber($("#var-th").value) ?? 10;
    busy(box, "Forge is comparing…");
    try { drawVariance(await call("variance", { files: S.files, budget: S.main, actual: A.actual, threshold: A.threshold })); } catch (err) { fail(box, err); }
  }
});

// ---------- Tools view: the MCP server's tools, run here ----------
const TOOLS = [
  ["Core", [["validate", "Check a model for formula errors, circular references and type mismatches."], ["calculate", "Calculate every formula, optionally under a named scenario."], ["audit", "Follow one variable's formula down to the inputs it reads."], ["report", "Groups with the same keys, side by side."]]],
  ["What-if", [["goal_seek", "Find the input value that makes an output hit a target."], ["break_even", "Find where an output reaches zero."], ["scenarios", "Run each probability-weighted scenario."], ["compare", "Put scenario results side by side."], ["sensitivity", "Vary one input over a range and watch an output."], ["tornado", "Rank inputs by how much they move one output."], ["variance", "Budget against actual, with favourable and unfavourable flags."]]],
  ["Uncertainty", [["simulate", "Monte Carlo with Latin hypercube sampling and a seed."], ["probability_seek", "Find the input that gives an output a chance of passing a threshold."], ["bootstrap", "Bootstrap confidence intervals from data."], ["decision_tree", "Backward induction: best path and expected value."], ["real_options", "Value the option to defer, expand or abandon."], ["bayesian", "Posterior probabilities in a Bayesian network."]]],
  ["Reference", [["functions", "The Excel-compatible functions, by category."], ["examples", "Runnable example models, by name."], ["schema", "The JSON Schema for Forge model files."]]],
];
function toolArgs(tool) {
  const A = defaults(), base = { files: "…", main: S.main };
  const first = inputs()[0], v = first ? first.value : 1;
  switch (tool) {
    case "functions": case "examples": case "schema": return {};
    case "calculate": return { ...base, scenario: S.scenario };
    case "audit": return { ...base, variable: A.out };
    case "compare": return { ...base, scenarios: S.structure.scenarios.slice(0, 2) };
    case "goal_seek": return { ...base, target: A.out, value: A.target ?? 0, vary: A.vary };
    case "break_even": return { ...base, output: A.out, vary: A.vary };
    case "sensitivity": return { ...base, vary: A.vary, range: [v * 0.8, v * 1.2, v * 0.1].map((x) => Number(x.toPrecision(4))).join(","), output: A.out };
    case "simulate": case "bootstrap": return { ...base, iterations: 1000, seed: 7 };
    case "probability_seek": return { ...base, goal: { output: A.out, threshold: "> 0", probability: 0.8 }, input: A.vary, min: Number((v * 0.5).toPrecision(4)), max: Number((v * 1.5).toPrecision(4)), iterations: 1000, seed: 7 };
    case "variance": return { files: "…", budget: S.main, actual: A.actual };
    default: return base;
  }
}
function renderTools() {
  const v = $("#view-mcp");
  if (!S.structure) { v.innerHTML = `<div class="msg bad">${esc(S.error || "No model")}</div>`; return; }
  v.innerHTML = `<p class="lead">Forge's MCP server gives an AI agent these tools. Here they run in this page, on the open model, with the same arguments (the files in place of paths) and the same answers.</p>` +
    TOOLS.map(([group, list]) => `<div class="eyebrow"><h2>${esc(group)}</h2><span class="tag">${list.length} tools</span></div><div class="card stack">${list.map(([name, desc]) => `
      <div class="row" style="padding:0"><button type="button" class="tool" data-tool="${name}" aria-expanded="${S.tools.open === name}"><span class="tool-name">forge_${name}</span><span class="pill live">runs here</span><span class="tool-desc">${esc(desc)}</span></button>
      ${S.tools.open === name ? `<div class="tool-body"><p class="sub">request</p><pre class="code">${esc(`call("${name}", ${JSON.stringify(toolArgs(name), null, 2)})`)}</pre>
        <div class="btns"><button type="button" class="btn primary" data-run="${name}">Run</button></div><div data-out="${name}">${S.tools.out && S.tools.out[0] === name ? S.tools.out[1] : ""}</div></div>` : ""}</div>`).join("")}</div>`).join("");
}
$("#view-mcp").addEventListener("click", async (e) => {
  const t = e.target.closest("[data-tool]");
  if (t) { S.tools.open = S.tools.open === t.dataset.tool ? null : t.dataset.tool; renderTools(); return; }
  const r = e.target.closest("[data-run]");
  if (r) {
    const name = r.dataset.run, box = $(`[data-out="${name}"]`), args = toolArgs(name);
    if (args.files) args.files = S.files;
    busy(box, "Running…");
    try {
      const out = JSON.stringify(await call(name, args), null, 2);
      S.tools.out = [name, `<p class="sub">answer</p><pre class="code">${esc(out.length > 6000 ? out.slice(0, 6000) + "\n…" : out)}</pre>`];
    } catch (err) { S.tools.out = [name, `<div class="msg bad">${esc(err.message)}</div>`]; }
    box.innerHTML = S.tools.out[1];
  }
});

// ---------- Files view ----------
function download(name, text, type = "text/yaml") {
  const a = document.createElement("a");
  a.href = URL.createObjectURL(new Blob([text], { type }));
  a.download = name;
  document.body.append(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 1000);
}
function csvOf(table) {
  const t = S.calc.tables[table];
  const q = (x) => (typeof x === "string" && /[",\n]/.test(x) ? `"${x.replace(/"/g, '""')}"` : String(x));
  const n = Math.max(0, ...Object.values(t.values).map((c) => c.length));
  return [t.columns.join(","), ...Array.from({ length: n }, (_, i) => t.columns.map((c) => q((t.values[c] || [])[i] ?? "")).join(","))].join("\n") + "\n";
}
function renderFiles() {
  const v = $("#view-files");
  const set = S.sets.find((x) => x.id === S.set);
  const edited = set && Object.keys(S.files).some((f) => S.files[f] !== set.files[f]);
  const included = new Set(S.structure ? S.structure.includes.map((i) => i.file) : []);
  v.innerHTML = `<div class="eyebrow"><h2>examples</h2><span class="tag">from forge's examples/</span></div>
    <div class="card stack">${S.sets.map((x) => `<div class="row"><button type="button" class="ex" data-set="${esc(x.id)}"><b>${esc(x.title)}</b>${x.id === S.set ? '<span class="pill live">open</span>' : ""}<span class="note">${esc(x.blurb)}</span></button></div>`).join("")}</div>
    <div class="eyebrow"><h2>this model's files</h2><span class="tag">${Object.keys(S.files).length}</span></div>
    <div class="card stack">${Object.keys(S.files).map((f) => `<div class="row file"><div><b>${esc(f)}</b><p class="note">${f === S.main ? "Main model" : included.has(f) ? "Included by the main model" : "Another model"}</p></div>
      ${f !== S.main && !included.has(f) ? `<button class="btn" type="button" data-main="${esc(f)}">Open</button>` : ""}<button class="btn" type="button" data-dl="${esc(f)}">Download</button></div>`).join("")}</div>
    ${S.calc && Object.keys(S.calc.tables).length ? `<div class="eyebrow"><h2>tables as CSV</h2><span class="tag">calculated</span></div><div class="card stack">${Object.keys(S.calc.tables).map((t) => `<div class="row file"><div><b>${esc(t)}.csv</b></div><button class="btn" type="button" data-csv="${esc(t)}">Download</button></div>`).join("")}</div>` : ""}
    <div class="eyebrow"><h2>your own model</h2></div>
    <div class="card row form"><label class="lab" for="imp">Open YAML files from this device (the main model and the files it includes)</label><input type="file" id="imp" accept=".yaml,.yml,text/yaml" multiple>
      <p class="note">They stay in this browser. Excel files come later (roadmap S8).</p></div>
    ${edited ? `<div class="btns"><button class="btn warn" type="button" id="reset">Undo all edits to ${esc(set.title)}</button></div>` : ""}`;
}
const vf = $("#view-files");
vf.addEventListener("click", async (e) => {
  const t = e.target;
  if (t.closest("[data-set]")) { openSet(t.closest("[data-set]").dataset.set); save(); show("model"); await load(); return; }
  if (t.dataset.main) { S.main = t.dataset.main; S.title = t.dataset.main; forget(); save(); show("model"); await load(); return; }
  if (t.dataset.dl) download(t.dataset.dl, S.files[t.dataset.dl]);
  if (t.dataset.csv) download(t.dataset.csv + ".csv", csvOf(t.dataset.csv), "text/csv");
  if (t.id === "reset") { openSet(S.set); save(); show("model"); await load(); }
});
vf.addEventListener("change", async (e) => {
  if (e.target.id !== "imp" || !e.target.files.length) return;
  const files = {};
  for (const f of e.target.files) files[f.name] = await f.text();
  const names = Object.keys(files);
  const main = names.find((n) => /^\s*_includes\s*:/m.test(files[n])) || names[0];
  Object.assign(S, { set: null, files, main, title: main });
  forget(); save(); show("model"); await load();
});

// ---------- routing ----------
const VIEWS = ["model", "yaml", "analyze", "mcp", "files"];
const RENDER = { model: () => { renderModel(); updateValues(); }, yaml: renderYaml, analyze: renderAnalyze, mcp: renderTools, files: renderFiles };
function show(tab) {
  S.tab = tab;
  for (const v of VIEWS) $("#view-" + v).hidden = v !== tab;
  $$(".tab").forEach((b) => (b.dataset.tab === tab ? b.setAttribute("aria-current", "page") : b.removeAttribute("aria-current")));
  RENDER[tab]();
  window.scrollTo(0, 0);
}
function renderAll() { RENDER[S.tab](); updateStatus(); }
$$(".tab").forEach((b) => b.addEventListener("click", () => show(b.dataset.tab)));

// ---------- start ----------
async function boot() {
  try {
    S.sets = await (await fetch("examples.json")).json();
  } catch (e) {
    S.error = "The examples could not be loaded."; S.sets = [];
  }
  const asked = new URLSearchParams(location.search).get("example");
  if (asked || !restore()) { if (S.sets.length) openSet(asked || S.sets[0].id); }
  S.yamlFile = S.main;
  show("model");
  try {
    S.version = await start();
  } catch (e) {
    S.error = e.message; renderAll(); finish(); return;
  }
  await load();
}
boot();
