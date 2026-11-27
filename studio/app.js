// Forge Studio (ADR-056): the YAML is the model, the cards show it, and every
// number on screen comes from Forge through forge-wasm (studio/forge.js).
// This file holds the page: state, views and events. It reads no YAML and
// calculates nothing; see studio/tests/no-engine.mjs.

import { call, start } from "./forge.js";
import { esc, highlightYaml, useFormulaRules } from "./text.js";
import { fmt, short, spark, sliderRange, readNumber, isPercent } from "./format.js";
import { tableCard, tournamentCard, pathHover } from "./games.js";
import { gameEditor, listenGameEdits } from "./gameedit.js";
import { templateCard, listenTemplates } from "./templates.js";
import { kindOf, questionsCard, setStrip, drawKinds } from "./sets.js";
import { ANALYSES, analysisSlot, drawAnalysis } from "./analyses.js"; import { bayesExtra } from "./bayesscenario.js";
import { toolArgs, toolsHtml, answerHtml } from "./tools.js";
import { helpButton, helpBody, suggestionStrip } from "./help.js";
import { isModelLink, listenLinks, linkCard, toBase64, fromBase64 } from "./link.js";
import { bandScheduler, listenDistributions, uncertainCount, uncertainText, uncertainNote, scenarioWhy, distributionOf, distributionsOf, distCard, drawBands } from "./uncertain.js";
import { scopeOf, namesFor, gameResults, functionsOf, suggest, apply, schemaOf, suggestKeys, formulaContext, suggestInFormula } from "./suggest.js";

const $ = (s, el = document) => el.querySelector(s);
const $$ = (s, el = document) => [...el.querySelectorAll(s)];
const STORE = "forge-studio-v3";

const S = {
  sets: [], set: null, files: {}, main: "", title: "", version: "",
  structure: null, scenarios: null, scenarioError: null, report: null, scenario: null, calc: null, games: null,
  error: null, generation: 0, open: new Set(), prev: {}, yamlFile: "", tab: "model",
  analyze: {}, tools: {}, kinds: {}, band: null,
  // The answers of a decision tree, real options, a bootstrap or a Bayesian
  // network the file holds (ADR-077); a network's nodes and what was seen
  answers: {}, bnNodes: null, evidence: {},
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
// The inputs an analysis can vary: numbers, not dates (ADR-060)
const inputs = () => (S.structure ? S.structure.scalars.filter((s) => s.formula == null && s.value != null && !s.date) : []);
const results = () => (S.structure ? S.structure.scalars.filter((s) => s.formula != null) : []);
// An uncertain input of an included file is in the answer's plan figures only (ADR-076)
const valueOf = (path) => (!S.calc ? undefined : path in S.calc.scalars ? S.calc.scalars[path] : S.calc.at_plan && path in S.calc.at_plan ? S.calc.at_plan[path] : undefined);
// The unit the model gives a scalar or a column (null when it has none), so
// that a number is shown as a percentage by its unit first (format.js)
function unitAt(path) {
  if (!S.structure || !path) return null;
  const entry = formulaOf(S.structure, String(path).replace(/ \(row \d+\)$/, ""));
  return entry && entry.unit ? entry.unit : null;
}
// A value with a format: is shown as Forge formats it (ADR-061); Studio adds
// no rule of its own then
const formattedAt = (path) => (S.calc && S.calc.formatted && path in S.calc.formatted ? S.calc.formatted[path] : undefined);
const fmtAt = (v, path) => fmt(v, leaf(String(path || "").replace(/ \(row \d+\)$/, "")), unitAt(path));
const shortAt = (v, path) => short(v, leaf(String(path || "").replace(/ \(row \d+\)$/, "")), unitAt(path));

// ---------- storage (this browser only) ----------
function save() {
  try { localStorage.setItem(STORE, JSON.stringify({ set: S.set, files: S.files, main: S.main, title: S.title })); } catch (e) { /* private mode */ }
}
// The welcome card shows on a first visit until it is dismissed (ADR-056
// addendum): a flag on this device, nothing sent anywhere
const WELCOME = "forge-studio-welcome-seen";
let welcomeSeen = (() => { try { return localStorage.getItem(WELCOME) === "1"; } catch (e) { return false; } })();
function setWelcome(seen) {
  welcomeSeen = seen;
  try { seen ? localStorage.setItem(WELCOME, "1") : localStorage.removeItem(WELCOME); } catch (e) { /* private mode: this visit only */ }
}
function welcomeCard() {
  if (welcomeSeen) return "";
  const models = modelsOf();
  if (models) return questionsCard(setOf().title, models, S.main, S.kinds);
  return `<section class="welcome card" id="welcome" aria-labelledby="welcome-title">
    <h2 id="welcome-title">Welcome to Forge Studio</h2>
    <p>This is a real financial model, calculated by Forge in your browser. Nothing you type leaves this device.</p>
    <ul>
      <li><b>Change an input.</b> Drag a slider or type a number, and every result below is calculated again.</li>
      <li><b>Open a formula.</b> Tap Edit on a result to see how it is calculated, and change it.</li>
      <li><b>Look around.</b> Grid shows the model as a spreadsheet, YAML shows the file itself, and Files has the other examples and your own files.</li>
    </ul>
    <button type="button" class="btn primary" id="welcome-done">Got it</button>
  </section>`;
}
function restore() {
  try {
    const saved = JSON.parse(localStorage.getItem(STORE) || "null");
    if (saved && saved.files && saved.main in saved.files) { Object.assign(S, saved); return true; }
  } catch (e) { /* nothing saved */ }
  return false;
}
// ---------- a set's models (ADR-077) ----------
const setOf = () => S.sets.find((x) => x.id === S.set);
// The open set's models as [file, short name, question], when it has several
// and they are still among the open files
function modelsOf() {
  const set = setOf();
  return set && set.models ? set.models.filter(([file]) => Object.hasOwn(S.files, file)) : null;
}
// The kind of each of the set's models, from Forge's `model` answer for its
// file; worked out once a set is opened, and drawn when each answer comes
async function loadKinds() {
  const models = modelsOf();
  if (!models) return;
  await Promise.all(models.filter(([file]) => !(file in S.kinds)).map(async ([file]) => {
    try { S.kinds[file] = kindOf(await call("model", { files: S.files, main: file })); } catch (e) { S.kinds[file] = "calculation"; }
  }));
  drawKinds(document, S.kinds);
}
const questionOf = (file) => ((modelsOf() || []).find(([f]) => f === file) || [])[2];
// Open one model of the set, as the strip, a question and Files do
async function openModel(file) {
  const set = setOf();
  S.main = file; S.title = set && file === set.main ? set.title : file;
  forget(); save(); show("model"); window.scrollTo(0, 0);
  await load();
}

// A set opens on its own main model, or on `model` when the set has that file
// (`?example=bagel&model=price_war.yaml`, as Files' Open button does)
function openSet(id, model) {
  const set = S.sets.find((x) => x.id === id) || S.sets[0];
  S.set = set.id; S.files = { ...set.files }; S.main = set.main; S.title = set.title; S.kinds = {};
  if (model && model !== set.main && Object.hasOwn(set.files, model)) { S.main = model; S.title = model; }
  forget();
}
// ---------- a link that carries a model after the # (ADR-080, link.js) ----------
// What a link or ?example= would replace, kept only in this browser
const ownFiles = () => Object.keys(S.files).filter((f) => !setOf() || S.files[f] !== setOf().files[f]);
const links = listenLinks({
  replaced: ownFiles, sheet: openSheet,
  example: (id) => (S.sets.find((x) => x.id === id) || S.sets[0] || { title: id }).title,
  openExample: (id, model) => { openSet(id, model); return shown(); },
  open: (link, now) => {
    Object.assign(S, { set: null, files: link.files, main: link.main, title: link.main, kinds: {} });
    forget();
    return now ? shown() : save();
  },
  model: () => ({ files: S.files, order: modelFiles() }),
});
async function shown() { save(); show("model"); window.scrollTo(0, 0); await load(); }
function forget() {
  S.scenario = null; S.open = new Set(); S.prev = {}; S.yamlFile = S.main; S.analyze = {}; S.tools = {}; S.answers = {}; S.bnNodes = null; S.evidence = {};
}

// ---------- Forge ----------
const request = (extra = {}) => ({ files: S.files, main: S.main, ...extra });

async function load() {
  const generation = ++S.generation;
  document.body.dataset.state = "loading";
  S.error = null; S.structure = null; S.calc = null; S.scenarios = null; S.scenarioError = null; S.report = null; S.games = null; S.band = null;
  try {
    const structure = await call("model", request());
    if (generation !== S.generation) return;
    S.structure = structure;
    // Why forge scenarios could not weigh them (none has a probability, or they do not sum to 1), shown on Analyze; an uncertain model at plan (ADR-076)
    if (structure.scenarios.length) S.scenarios = await call("scenarios", request({ plan: true })).catch((e) => { S.scenarioError = e.message; return null; });
    S.report = await call("report", request()).catch(() => null);
    await loadFunctions(); // a distribution's card names its parameters as Forge's list does
    if (S.scenario && !structure.scenarios.includes(S.scenario)) S.scenario = null;
  } catch (e) {
    if (generation !== S.generation) return;
    S.error = e.message;
  }
  if (generation !== S.generation) return;
  renderAll();
  await recalc();
  await loadKinds();
}

async function recalc() {
  if (!S.structure) { finish(); return; }
  const generation = ++S.generation;
  try {
    // An uncertain model at its plan figures; any other as it is (ADR-076)
    const calc = await call("calculate", request({ scenario: S.scenario, plan: true }));
    // The games' results, worked out again with the inputs (ADR-068)
    const games = S.structure.games && S.structure.games.length
      ? await call("game", request({ scenario: S.scenario })).catch((e) => ({ error: e.message })) : null;
    const answers = await runAnalyses();
    if (generation !== S.generation) return;
    S.calc = calc; S.games = games; S.answers = answers; S.error = null;
    scheduleBands();
  } catch (e) {
    if (generation !== S.generation) return;
    S.calc = null; S.games = null; S.error = e.message;
  }
  updateValues();
  finish();
}
// The ranges of an uncertain model's outputs (ADR-076, studio/uncertain.js)
const scheduleBands = bandScheduler(S, () => call("simulate", request({ scenario: S.scenario })), () => updateValues());
function finish() {
  updateStatus();
  document.body.dataset.state = S.error ? "error" : "ready";
  document.body.dataset.generation = S.generation;
}

// The file's other analyses, each by its tool; a network with what was seen,
// and its nodes and states from its answer with nothing seen
async function runAnalyses() {
  const answers = {};
  for (const section of S.structure.analyses.filter((a) => ANALYSES[a])) {
    const tool = ANALYSES[section].tool;
    try {
      if (tool === "bayesian" && !S.bnNodes) S.bnNodes = Object.values((await call(tool, request())).queries).map(({ name, states }) => ({ name, states }));
      const extra = tool === "bayesian" ? bayesExtra(S) : {};
      answers[section] = await call(tool, request(extra));
    } catch (e) { answers[section] = { error: e.message }; }
  }
  return answers;
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
  $("#status-text").textContent = S.error ? "Problem" : uncertainCount(S.calc) ? uncertainText(uncertainCount(S.calc)) : p.length ? `${p.length} warning${p.length > 1 ? "s" : ""}` : S.version ? `Forge ${S.version}` : "Forge";
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
// ---------- help (ADR-077, studio/help.js): a "?" in each slot, a sheet on a tap ----------
let HELP = null;
// A group of scalars is helped as inputs when it has any, as results otherwise
const helpSlot = (list) => `<span data-help-slot="${list.some((s) => s.formula == null) ? "inputs" : "results"}"></span>`;
function fillHelp(root = document) {
  for (const slot of root.querySelectorAll("[data-help-slot]")) slot.outerHTML = helpButton(HELP, slot.dataset.helpSlot);
}
document.addEventListener("click", async (e) => {
  const q = e.target.closest("[data-help]");
  if (q && HELP) { openSheet(HELP[q.dataset.help].title, helpBody(HELP[q.dataset.help], S.version)); return; }
  const ex = e.target.closest("#sheet [data-example]");
  if (ex) { $("#sheet").hidden = true; openSet(ex.dataset.example, ex.dataset.exampleModel); await shown(); }
});
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

// `idp`: the start of each field's id, so that a card drawn twice (in a
// game's "what moves it" too) keeps ids of its own
function scalarCards(list, idp = "in-") {
  let h = "";
  for (const s of list.filter((x) => x.formula == null && x.date)) {
    // A date: typed as 2026-10-31, with no slider (ADR-060)
    const id = idp + s.path.replace(/[^A-Za-z0-9_-]/g, "_");
    h += `<div class="row" data-card="${esc(s.path)}" data-path="${esc(s.path)}"${s.path.startsWith("@") ? "" : ` data-scalar="${esc(s.path)}"`}>
      <div class="inp-head"><label class="inp-name" for="${id}">${esc(leaf(s.path))}</label><span class="inp-hint" data-hint></span>
      <input class="field" id="${id}" data-input="${esc(s.path)}" data-date="1" inputmode="numeric" autocomplete="off" placeholder="yyyy-mm-dd" value="${esc(s.value ?? "")}"></div>
      <div class="slide-row">${deleteButton(s.path, true)}</div>
      ${s.notes ? `<div class="note">${esc(s.notes)}</div>` : ""}
      <div class="by-scn" data-byscn hidden></div><div class="err-msg" data-err hidden></div></div>`;
  }
  for (const s of list.filter((x) => x.formula == null && !x.date)) {
    const r = sliderRange(s.value, leaf(s.path), s.unit);
    const id = idp + s.path.replace(/[^A-Za-z0-9_-]/g, "_");
    h += `<div class="row" data-card="${esc(s.path)}" data-path="${esc(s.path)}"${s.path.startsWith("@") ? "" : ` data-scalar="${esc(s.path)}"`}>
      <div class="inp-head"><label class="inp-name" for="${id}">${esc(leaf(s.path))}${unitOf(s)}</label><span class="inp-hint" data-hint></span>
      <input class="field" id="${id}" data-input="${esc(s.path)}" inputmode="decimal" autocomplete="off" value="${s.value ?? ""}"></div>
      <div class="slide-row"><input type="range" data-range="${esc(s.path)}" min="${r.min}" max="${r.max}" step="${r.step}" value="${s.value ?? r.min}" aria-label="${esc(leaf(s.path))}">${deleteButton(s.path, true)}</div>
      ${s.better ? `<div class="note">better ${esc(s.better)}</div>` : ""}${s.notes ? `<div class="note">${esc(s.notes)}</div>` : ""}
      <div class="slide-row"><button type="button" class="edit-btn" data-uncertain="${esc(s.path)}" aria-label="Make ${esc(s.path)} uncertain">Make uncertain</button></div>
      <div class="by-scn" data-byscn hidden></div><div class="err-msg" data-err hidden></div></div>`;
  }
  for (const s of list.filter((x) => x.formula != null)) {
    const d = distributionOf(s.formula);
    if (d) { h += distCard(s, d, distributionsOf(functionList), { name: leaf(s.path), unit: unitOf(s), deleteButton: deleteButton(s.path, true), editButton: editButton(s.path), idp: idp.replace(/^in-/, "dp-") }); continue; }
    const calculated = !s.path.startsWith("@");
    h += `<div class="row" data-card="${esc(s.path)}" data-path="${esc(s.path)}"${calculated ? ` data-scalar="${esc(s.path)}"` : ""}><div class="res-top"><div class="res-name">${esc(leaf(s.path))}${unitOf(s)}</div>${deleteButton(s.path, true)}</div>
      <div class="res-val" data-val>${calculated ? "" : '<span class="note">used by the model</span>'}</div>
      <div class="expect note" data-expect hidden></div>
      <div class="formula-row" data-fslot="${esc(s.path)}"><div class="formula">${esc(s.formula)}</div>${editButton(s.path)}</div></div>`;
  }
  return h;
}
const editButton = (path, label = "Edit") => `<button type="button" class="edit-btn" data-edit="${esc(path)}" aria-label="Edit the formula of ${esc(path)}"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M4 20h4L19 9l-4-4L4 16z"/><path d="M13.5 6.5l4 4"/></svg>${esc(label)}</button>`;
// `icon`: only the bin, so that the card keeps its height (the column's
// buttons, shown when it is open, keep the word)
const deleteButton = (path, icon = false) => `<button type="button" class="edit-btn del-btn${icon ? " icon" : ""}" data-del="${esc(path)}" aria-label="Delete ${esc(path)}"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M5 7h14M10 7V4h4v3M7 7l1 13h8l1-13"/></svg>${icon ? "" : "Delete"}</button>`;
// The Add button of a group of scalars (list: its scalars) or of a table
// (list null), in the main file (ns null) or an included one
const addKey = (ns, group) => `${ns ? "@" + ns + "." : ""}${group || ""}`;
function addButton(ns, group, list) {
  // At the top level a new scalar goes after the last one shown
  const after = !group && list ? leaf(list[list.length - 1].path) : "";
  return `<button type="button" class="edit-btn add-btn" data-add="${esc(addKey(ns, group))}" data-ns="${esc(ns || "")}" data-group="${esc(group || "")}" data-after="${esc(after)}" data-kind="${list ? "scalar" : "column"}" aria-label="Add to ${esc(group || "the top level")}"><svg viewBox="0 0 24 24" aria-hidden="true"><path d="M12 5v14M5 12h14"/></svg>Add</button>`;
}
const addSlot = (ns, group) => `<div class="card add-form" data-addslot="${esc(addKey(ns, group))}" hidden></div>`;
function tagOf(list) {
  const i = list.some((s) => s.formula == null), f = list.some((s) => s.formula != null);
  return i && f ? "inputs and results" : i ? "inputs" : "results";
}
function tableCards(tables, inc = "") {
  let h = "";
  for (const t of tables) {
    const rows = Math.max(0, ...t.columns.map((c) => c.rows || 0));
    const ns = /^@([^.]+)\./.exec(t.name), tname = t.name.replace(/^@[^.]+\./, "");
    h += `<div class="eyebrow"><h2>${esc(tname)}</h2><span class="tag">table${rows ? ` · ${rows} rows` : ""}</span>${inc}<span data-help-slot="tables"></span>${addButton(ns && ns[1], tname, null)}</div>${addSlot(ns && ns[1], tname)}<div class="card stack">`;
    for (const c of t.columns) {
      const key = `${t.name}.${c.name}`;
      const open = S.open.has(key);
      h += `<div class="row" style="padding:0" data-card="${esc(key)}" data-table="${esc(t.name)}" data-column="${esc(c.name)}"><button type="button" class="col" data-col="${esc(key)}" aria-expanded="${open}">
        <span class="col-name">${esc(c.name)}${c.unit ? ` <span class="unit">${esc(c.unit)}</span>` : ""}</span><span data-spark></span>
        ${c.formula ? `<span class="formula">${esc(c.formula)}</span>` : ""}<span class="col-meta" data-meta></span></button>
        <div class="vals" data-vals ${open ? "" : "hidden"}></div>
        <div class="col-edit" ${c.formula ? `data-fslot="${esc(key)}"` : ""} ${open ? "" : "hidden"}>${c.formula ? editButton(key, "Edit the formula") : ""}${deleteButton(key)}</div></div>`;
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
    s.rows.map((r) => `<tr><th scope="row">${esc(r.group)}</th>${s.keys.map((k) => { const v = r.values[k]; return `<td class="${typeof v === "number" && v < 0 ? "neg" : ""}">${esc(fmtAt(v, `${r.group}.${k}`))}</td>`; }).join("")}</tr>`).join("") +
    `</tbody></table></div>`).join("");
}

function renderModel() {
  const v = $("#view-model");
  if (!S.structure) {
    v.innerHTML = `<div class="msg bad">${esc(S.error || "Forge is starting…")}</div><p class="note">Fix it in the YAML tab, or open another model from Files.</p>`;
    return;
  }
  const models = modelsOf();
  let h = welcomeCard() + (models ? setStrip(models, S.main, S.kinds) : "") + `<div class="msg bad" id="model-error" hidden></div><div class="msg uncertain" id="model-uncertain" hidden></div>`;
  h += S.structure.analyses.filter((a) => ANALYSES[a]).map(analysisSlot).join("");
  if (S.structure.scenarios.length) {
    const info = (n) => (S.scenarios ? S.scenarios.scenarios.find((s) => s.name === n) : null);
    h += `<div class="eyebrow"><h2>scenarios</h2><span class="tag">tap to apply</span><span data-help-slot="scenarios"></span></div>
      <div class="chips" role="group" aria-label="Scenarios"><button type="button" class="chip" data-scn="" aria-pressed="${S.scenario === null}">plan</button>${S.structure.scenarios.map((n) => {
        const p = info(n) && info(n).probability;
        return `<button type="button" class="chip" data-scn="${esc(n)}" aria-pressed="${n === S.scenario}">${esc(n)}${p != null ? `<small>${Math.round(p * 100)}%</small>` : ""}</button>`;
      }).join("")}</div><p class="scn-desc" id="scn-desc"></p>`;
  }
  h += reportCards();
  h += gameCards();
  for (const [g, list] of groupsOf(S.structure.scalars)) {
    h += `<div class="eyebrow"><h2>${esc(g || "top level")}</h2><span class="tag">${tagOf(list)}</span>${helpSlot(list)}${addButton(null, g, list)}</div>${addSlot(null, g)}<div class="card stack">${scalarCards(list)}</div>`;
  }
  h += tableCards(S.structure.tables);
  // An included file's groups are folded under one line (ADR-076 decision 5):
  // the model shown is the one open, not the files it reads
  for (const inc of S.structure.includes) {
    const badge = `<span class="inc">@${esc(inc.as)} · ${esc(inc.file)}</span>`;
    const groups = groupsOf(inc.scalars);
    const count = (n, one) => `${n} ${one}${n === 1 ? "" : "s"}`;
    h += `<details class="inc-fold" data-incfold="${esc(inc.as)}"${S.open.has("inc:" + inc.as) ? " open" : ""}><summary>from ${esc(inc.file)}: ${[count(groups.size, "group"), ...(inc.tables.length ? [count(inc.tables.length, "table")] : [])].join(", ")}</summary>`;
    for (const [g, list] of groups) {
      h += `<div class="eyebrow"><h2>${esc(g || "top level")}</h2><span class="tag">${tagOf(list)}</span>${badge}${helpSlot(list)}${addButton(inc.as, g, list)}</div>${addSlot(inc.as, g)}<div class="card stack">${scalarCards(list)}</div>`;
    }
    h += tableCards(inc.tables, badge) + `</details>`;
  }
  v.innerHTML = h;
  S.prev = {};
  // The open model's chip in view, however far along the strip it is
  const strip = $(".set-strip", v), on = strip && $('[aria-pressed="true"]', strip);
  if (on) strip.scrollLeft = on.offsetLeft - strip.offsetLeft - (strip.clientWidth - on.offsetWidth) / 2;
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

// What the file, or the scenario applied, states this result should be (expect:, ADR-074), and whether Forge gives it: both from Forge's calculate
function showExpected(card, path) {
  const el = $("[data-expect]", card);
  if (!el) return;
  const check = S.calc && (S.calc.expectations || []).find((e) => e.path === path);
  el.hidden = !check;
  if (!check) return;
  const stated = `${fmtAt(check.expected, path)}${check.formula ? ` (${check.formula})` : ""}`;
  el.textContent = check.met ? `As expect: states, ${stated}` : `expect: states ${stated}, and Forge gives ${fmtAt(check.got, path)}`;
  el.classList.toggle("bad", !check.met);
  card.dataset.expected = check.met ? "met" : "differs";
}

function updateValues() {
  if (!S.structure) { renderModel(); return; }
  const problem = $("#model-error");
  if (problem) {
    problem.hidden = !S.error;
    problem.textContent = S.error || "";
  }
  const un = $("#model-uncertain"), n = uncertainCount(S.calc);
  if (un) { un.hidden = !n; un.innerHTML = n ? uncertainNote(n, S.band) : ""; }
  const d = $("#scn-desc");
  if (d) {
    const info = S.scenarios && S.scenarios.scenarios.find((s) => s.name === S.scenario);
    d.textContent = S.scenario === null ? "The plan as written in the file." : info && info.description ? info.description : "";
  }
  // The games first: a game's "what moves it" has input cards of its own
  drawGames();
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
      if (r && document.activeElement !== r && shown != null) r.value = shown;
      f.disabled = forced;
      if (r) r.disabled = forced;
      b.hidden = !forced;
      if (forced) b.textContent = `Set by the ${S.scenario} scenario. The plan has ${fmtAt(s.value, path)}.`;
      const formatted = formattedAt(path);
      $("[data-hint]", card).textContent = formatted !== undefined ? formatted : shown != null && isPercent(leaf(path), unitAt(path), shown) ? fmtAt(shown, path) : "";
    } else {
      const el = $("[data-val]", card);
      el.textContent = S.calc ? formattedAt(path) ?? fmtAt(v, path) : "Not calculated";
      el.classList.toggle("neg", typeof v === "number" && v < 0);
      showExpected(card, path);
    }
  }
  for (const card of $$("#view-model [data-table]")) {
    const t = S.calc && S.calc.tables[card.dataset.table];
    const vals = t ? t.values[card.dataset.column] || [] : [];
    const name = card.dataset.column, meta = $("[data-meta]", card), sp = $("[data-spark]", card), list = $("[data-vals]", card);
    pulse(card, card.dataset.card, JSON.stringify(vals));
    // A column of numbers may have empty cells (null, ADR-059): the spark and
    // the range are of its numbers
    const nums = vals.filter((x) => x !== null);
    const numbers = nums.length && nums.every((x) => typeof x === "number");
    sp.innerHTML = numbers ? spark(nums) : "";
    meta.innerHTML = !vals.length ? "<span>Not calculated</span>"
      : numbers ? `<span>${esc(shortAt(Math.min(...nums), card.dataset.card))} to ${esc(shortAt(Math.max(...nums), card.dataset.card))}</span><span>last ${esc(fmtAt(vals[vals.length - 1], card.dataset.card))}</span>`
      : `<span>${esc(vals.slice(0, 3).map((x) => fmtAt(x, card.dataset.card)).join(", "))}${vals.length > 3 ? ", …" : ""}</span><span>${vals.length} rows</span>`;
    if (!list.hidden) {
      const labels = rowLabels(card.dataset.table);
      list.innerHTML = vals.map((x, i) => `<span><i>${esc(labels[i] ?? i + 1)}</i>${esc(fmtAt(x, card.dataset.card))}</span>`).join("");
    }
  }
  drawBands($("#view-model"), S.band && S.band.answer, shortAt, S.open);
  for (const slot of $$("#view-model [data-analysis]")) {
    const answer = S.answers[slot.dataset.analysis];
    if (answer) drawAnalysis(slot, slot.dataset.analysis, answer, S.bnNodes, S.evidence, S.scenario);
  }
  fillHelp($("#view-model"));
}
// The templates Forge serves for "add a game" (ADR-076 decision 6)
let templates = [];
// ---------- games (ADR-068): forge game's answer, drawn ----------
// Every number here is in the answer of the `game` tool; the page only lays
// them out. A cell's payoffs and an equilibrium's chances are also kept in
// data- attributes, in full, for the screen test (tests/games.mjs).
function gameCards() {
  const names = S.structure.games || [];
  if (!names.length && !templates.length) return "";
  return `<div class="eyebrow" id="games"><h2>games</h2><span class="tag">${names.length ? "forge game · moves with the inputs" : "none yet"}</span><span data-help-slot="games"></span></div>` +
    names.map((n) => `<div class="card row game" data-game="${esc(n)}"><div data-gamebody><p class="note">Forge is working out ${esc(n)}…</p></div><div data-moves></div></div>`).join("") + templateCard(templates);
}
function drawGames() {
  for (const card of $$("#view-model [data-game]")) {
    const r = S.games && !S.games.error ? S.games.find((g) => g.game === card.dataset.game) : null;
    $("[data-gamebody]", card).innerHTML = (r ? (r.kind === "tournament" ? tournamentCard(r) : tableCard(r))
      : `<div class="msg bad">${esc(S.games && S.games.error ? S.games.error : S.error || "Not calculated")}</div>`) + resultsList(card.dataset.game) + (r ? gameEditor(r, S.open.has("gedit:" + r.game)) : "");
    drawMoves(card, r);
    pulse(card, "game:" + card.dataset.game, JSON.stringify(r));
  }
}
// What moves a game (ADR-076 decision 5): the input cards of the inputs
// Forge says its numbers read, here and in their own group alike. Drawn again
// only when the list changes, so that a field being typed in stays.
function drawMoves(card, r) {
  const slot = $("[data-moves]", card), byPath = new Map(allScalars().map((s) => [s.path, s]));
  const shown = (r && r.inputs ? r.inputs : []).map((p) => byPath.get(p)).filter(Boolean);
  const key = shown.map((s) => s.path).join(" ");
  if (slot.dataset.list === key) return;
  slot.dataset.list = key;
  const g = card.dataset.game;
  slot.innerHTML = shown.length ? `<details class="moves" data-movesof="${esc(g)}"${S.open.has("shut-moves:" + g) ? "" : " open"}><summary>what moves it: ${shown.length} input${shown.length === 1 ? "" : "s"}</summary><div class="stack">${scalarCards(shown, `mv-${g.replace(/[^A-Za-z0-9_-]/g, "_")}-`)}</div></details>` : "";
  // data-scalar names the one card of each scalar that the screen tests read
  for (const copy of $$("[data-scalar]", slot)) copy.removeAttribute("data-scalar");
}
// The game's results as `calculate` answers them, under the names a formula
// reads them by (stag_hunt.even_mix.me.stag), each with its value in full
// for the screen test
function resultsList(game) {
  const paths = gameResults(S.structure, S.calc).filter((p) => p.startsWith(game + "."));
  if (!paths.length) return "";
  const open = S.open.has("results:" + game);
  return `<details class="results" data-results="${esc(game)}"${open ? " open" : ""}><summary>${paths.length} results a formula can read</summary><table class="report"><tbody>` +
    paths.map((p) => { const v = valueOf(p); return `<tr data-scalar="${esc(p)}" data-value="${esc(v === undefined || v === null ? "" : String(v))}"><th scope="row" class="mono">${esc(p)}</th><td>${esc(fmt(v))}</td></tr>`; }).join("") +
    `</tbody></table></details>`;
}
let typing = 0;
// An input's value written by Forge's `edit`, one change after another so
// that a fast typist's changes are never made on stale files
let writing = Promise.resolve();
function setInput(path, value) {
  writing = writing.then(async () => {
    let answer;
    try {
      answer = await call("edit", { files: S.files, main: S.main, changes: [{ op: "set_value", path, value }] });
    } catch (e) {
      openSheet("Not changed", `<p>${esc(e.message)}</p>`);
      return;
    }
    S.files = { ...S.files, ...answer.files };
    for (const file of Object.keys(answer.files)) refreshEditor(file);
    const s = allScalars().find((x) => x.path === path);
    if (s) s.value = value;
    save();
    clearTimeout(typing);
    typing = setTimeout(recalc, 120);
  });
}
// The generation under the pointer on a game's dynamics chart
$("#view-model").addEventListener("pointermove", pathHover);
$("#view-model").addEventListener("pointerdown", pathHover);
// A game's list of results stays open or shut while the games are drawn again
$("#view-model").addEventListener("toggle", (e) => {
  const d = e.target.closest && e.target.closest("[data-results], [data-gedit], details[data-band], [data-movesof], [data-incfold]");
  if (!d) return;
  // What moves a game is open until it is shut; an included file, shut until opened
  if (d.dataset.movesof) { if (d.open) S.open.delete("shut-moves:" + d.dataset.movesof); else S.open.add("shut-moves:" + d.dataset.movesof); return; }
  const k = d.dataset.results ? "results:" + d.dataset.results : d.dataset.gedit ? "gedit:" + d.dataset.gedit : d.dataset.incfold ? "inc:" + d.dataset.incfold : "band:" + d.dataset.band;
  if (d.open) S.open.add(k); else S.open.delete(k);
}, true);
$("#view-model").addEventListener("input", (e) => {
  const t = e.target, path = t.dataset.input || t.dataset.range;
  if (!path) return;
  if (t.dataset.date) {
    // A whole date only, so the file never holds half of one
    if (/^\d{4}-\d{2}-\d{2}$/.test(t.value.trim())) setInput(path, t.value.trim());
    return;
  }
  const n = readNumber(t.value);
  if (n === null) return;
  const card = t.closest("[data-card]");
  if (t.dataset.range) $("[data-input]", card).value = n; else $("[data-range]", card).value = n;
  setInput(path, n);
});
$("#view-model").addEventListener("click", (e) => {
  if (e.target.id === "welcome-done") { setWelcome(true); $("#welcome").remove(); return; }
  // A state seen on a Bayesian network, or, tapped again, seen no more
  const ev = e.target.closest("[data-ev]");
  if (ev) {
    const node = ev.dataset.ev, seen = S.evidence[node] === ev.dataset.state;
    S.evidence = Object.fromEntries([...Object.entries(S.evidence).filter(([n]) => n !== node), ...(seen ? [] : [[node, ev.dataset.state]])]);
    recalc();
    return;
  }
  const model = e.target.closest("[data-model]");
  if (model) {
    // A question answered is the welcome card read; the strip stays
    if (model.closest("#welcome")) setWelcome(true);
    if (model.dataset.model !== S.main) openModel(model.dataset.model);
    return;
  }
  const sug = e.target.closest("[data-sug]");
  if (sug) { takeSuggestion(sug); return; }
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
    const edit = col.parentElement.querySelector(".col-edit");
    if (edit) edit.hidden = !S.open.has(k);
    updateValues();
    return;
  }
  const edit = e.target.closest("[data-edit]");
  if (edit) { openFormulaEditor(edit.dataset.edit); return; }
  const add = e.target.closest("[data-add]");
  if (add) { openAddForm(add); return; }
  const kind = e.target.closest("[data-akind]");
  if (kind) { setAddKind(kind.closest("[data-addslot]"), kind.dataset.akind); return; }
  const addCancel = e.target.closest("[data-acancel]");
  if (addCancel) { closeAddForm(addCancel.closest("[data-addslot]")); return; }
  const addSave = e.target.closest("[data-asave]");
  if (addSave) { saveAdd(addSave.closest("[data-addslot]")); return; }
  const uncertain = e.target.closest("[data-uncertain]");
  if (uncertain) {
    uncertain.disabled = true;
    editModel([{ op: "make_uncertain", path: uncertain.dataset.uncertain }]).then((why) => {
      if (why) { openSheet("Not changed", `<p>${esc(why)}</p>`); uncertain.disabled = false; }
    });
    return;
  }
  const del = e.target.closest("[data-del]");
  if (del) { askDelete(del); return; }
  const delYes = e.target.closest("[data-delyes]");
  if (delYes) { deleteElement(delYes.dataset.delyes); return; }
  const delNo = e.target.closest("[data-delno]");
  if (delNo) { const ask = delNo.closest(".del-ask"); ask.previousElementSibling.hidden = false; ask.remove(); return; }
  const cancel = e.target.closest("[data-fcancel]");
  if (cancel) { closeFormulaEditor(cancel.dataset.fcancel); return; }
  const keep = e.target.closest("[data-fsave]");
  if (keep) saveFormula(keep.dataset.fsave);
});
// A distribution card's kind or parameter changed: its formula is written as an edited formula is
listenDistributions($("#view-model"), () => distributionsOf(functionList), (path, formula) => writeFormula(path, formula),
  (path) => editModel([{ op: "make_certain", path }]));
listenGameEdits($("#view-model"), editModel);
listenTemplates($("#view-model"), () => templates, editModel);
$("#view-model").addEventListener("keydown", (e) => {
  const slot = e.target.closest("[data-addslot]");
  if (slot && e.target.matches("input")) {
    if (e.key === "Enter") { e.preventDefault(); saveAdd(slot); }
    if (e.key === "Escape") closeAddForm(slot);
    return;
  }
  const input = e.target.closest("[data-finput]");
  if (!input) return;
  if (e.key === "Enter") { e.preventDefault(); saveFormula(input.dataset.finput); }
  if (e.key === "Escape") closeFormulaEditor(input.dataset.finput);
});

// ---------- suggestions while a formula is typed (roadmap S10) ----------
// A strip of names and functions above the field, from the model and from
// Forge's own function list (suggest.js); a tap inserts exactly its text.
let functionList = null, functionsLoading = null;
function loadFunctions() {
  if (!functionsLoading) {
    functionsLoading = call("functions", {}).then((a) => { functionList = functionsOf(a); }).catch(() => { functionList = []; });
  }
  return functionsLoading;
}
const formulaFields = "[data-finput], [data-aform]";
function scopeFor(input) {
  if (input.dataset.finput) return scopeOf(S.structure, input.dataset.finput);
  const { ns, group, kind } = input.closest("[data-addslot]").dataset;
  return kind === "column" ? { ns: ns || null, group: null, table: group || null } : { ns: ns || null, group: group || null, table: null };
}
function showSuggestions(input) {
  const strip = input.parentElement.querySelector("[data-suggest]");
  if (!strip || !S.structure) return;
  if (!functionList) loadFunctions().then(() => { if (input.isConnected) showSuggestions(input); });
  const list = suggest({ names: namesFor(S.structure, scopeFor(input), gameResults(S.structure, S.calc)), functions: functionList || [] }, input.value, input.selectionStart ?? input.value.length);
  strip.innerHTML = suggestionStrip(list, (x, i) => `<button type="button" class="sug ${x.kind}" data-sug="${i}" title="${esc(x.title)}">${esc(x.label)}</button>`);
  strip._list = list;
  strip.hidden = !list.length;
}
function takeSuggestion(button) {
  const strip = button.closest("[data-suggest]"), input = strip.parentElement.querySelector("input, textarea");
  const r = apply(input.value, strip._list[Number(button.dataset.sug)]);
  input.value = r.text;
  input.focus();
  input.setSelectionRange(r.caret, r.caret);
  if (input.id === "yaml-editor") input.dispatchEvent(new Event("input", { bubbles: true })); // paint, and apply on a wide screen
  else showSuggestions(input);
}

// Keys of the model format in the YAML editor (roadmap S12): from the schema
// Forge returns for the file's own format version (the `schema` tool)
const schemas = new Map();
function loadSchema(text) {
  const m = /^_forge_version:\s*["']?(\d+\.\d+\.\d+)/m.exec(text);
  const version = m ? `v${m[1]}` : "latest";
  if (!schemas.has(version)) {
    schemas.set(version, (async () => {
      try {
        let v = version;
        if (v === "latest") {
          const list = JSON.parse(await call("schema", {})).available_versions;
          v = list[list.length - 1].version;
        }
        return schemaOf(await call("schema", { version: v }));
      } catch (e) { return null; }
    })());
  }
  return schemas.get(version);
}
let keysFor = 0;
async function showKeySuggestions(editor) {
  const strip = $("#view-yaml [data-suggest]");
  if (!strip) return;
  const token = ++keysFor;
  const text = editor.value, caret = editor.selectionStart ?? text.length;
  let list;
  if (formulaContext(text, caret)) {
    // In a formula: the names and functions of the cards, not keys
    await loadFunctions();
    if (token !== keysFor || !editor.isConnected) return;
    const part = S.structure && (S.yamlFile === S.main || S.structure.includes.some((i) => i.file === S.yamlFile));
    list = suggestInFormula({ structure: part ? S.structure : null, functions: functionList || [], file: S.yamlFile, results: gameResults(S.structure, S.calc) }, text, caret) || [];
  } else {
    const schema = await loadSchema(text);
    if (token !== keysFor || !editor.isConnected) return;
    list = suggestKeys(schema, text, caret);
  }
  strip.innerHTML = suggestionStrip(list, (x, i) => `<button type="button" class="sug ${x.kind}${x.used ? " used" : ""}" data-sug="${i}" title="${esc(x.title)}">${esc(x.label)}</button>`);
  strip._list = list;
  strip.hidden = !list.length;
}
for (const type of ["input", "keyup", "click", "focusin"]) {
  $("#view-model").addEventListener(type, (e) => { if (e.target.matches && e.target.matches(formulaFields)) showSuggestions(e.target); });
}
// A tap on a suggestion must not take the focus (and the keyboard) from the field
$("#view-model").addEventListener("mousedown", (e) => { if (e.target.closest("[data-sug]")) e.preventDefault(); });

// ---------- editing a formula on its card (roadmap S7) ----------
// The new text is kept only when Forge accepts it: the model must validate
// (or simulate, for a model with distributions), and Forge must read back
// exactly this formula and nothing else changed. Otherwise Forge's message is
// shown and the file is as it was.
function formulaOf(structure, path) {
  const scalars = [...structure.scalars, ...structure.includes.flatMap((i) => i.scalars)];
  const s = scalars.find((x) => x.path === path);
  if (s) return s;
  for (const t of [...structure.tables, ...structure.includes.flatMap((i) => i.tables)]) {
    const c = t.columns.find((x) => `${t.name}.${x.name}` === path);
    if (c) return c;
  }
  return null;
}
const slotOf = (path) => $(`#view-model [data-fslot="${CSS.escape(path)}"]`);
function openFormulaEditor(path) {
  const slot = slotOf(path), entry = formulaOf(S.structure, path);
  if (!slot || !entry) return;
  slot.dataset.before = slot.innerHTML;
  slot.classList.add("editing");
  slot.innerHTML = `<div class="fedit"><label class="lab" for="f-${esc(path)}">Formula of ${esc(path)}</label>
    <div class="suggest" data-suggest hidden aria-label="Suggestions"></div>
    <input class="txt" id="f-${esc(path)}" data-finput="${esc(path)}" value="${esc(entry.formula)}" autocomplete="off" autocapitalize="off" autocorrect="off" spellcheck="false">
    <div class="btns"><button type="button" class="btn primary" data-fsave="${esc(path)}">Check and save</button><button type="button" class="btn" data-fcancel="${esc(path)}">Cancel</button></div>
    <div class="msg bad" data-fmsg hidden></div></div>`;
  const input = $("[data-finput]", slot);
  input.focus();
  input.setSelectionRange(input.value.length, input.value.length);
}
function closeFormulaEditor(path) {
  const slot = slotOf(path);
  if (!slot || slot.dataset.before === undefined) return;
  slot.innerHTML = slot.dataset.before;
  delete slot.dataset.before;
  slot.classList.remove("editing");
}
async function saveFormula(path) {
  const slot = slotOf(path);
  if (!slot) return;
  const input = $("[data-finput]", slot), msg = $("[data-fmsg]", slot), button = $("[data-fsave]", slot);
  let formula = input.value.trim();
  if (!formula.startsWith("=")) formula = "=" + formula;
  const entry = formulaOf(S.structure, path);
  if (!entry) return;
  if (formula === entry.formula) { closeFormulaEditor(path); return; }
  button.disabled = true;
  const why = await writeFormula(path, formula);
  if (why) { msg.hidden = false; msg.textContent = why; button.disabled = false; }
}
// `formula` written by Forge's `edit`: the reason when it is refused, and the
// files as they were
const writeFormula = (path, formula) => editModel([{ op: "set_formula", path, formula }]);

// A change Forge writes itself (ADR-075): `edit` answers the new text of each
// file it changed, kept only when Forge reads the model back as asked;
// otherwise why not, and the files are as they were
async function editModel(changes) {
  let answer;
  try {
    answer = await call("edit", { files: S.files, main: S.main, changes });
  } catch (e) {
    return e.message;
  }
  S.files = { ...S.files, ...answer.files };
  for (const file of Object.keys(answer.files)) refreshEditor(file);
  save();
  await load();
  return null;
}

// ---------- adding and deleting an element (roadmap S9) ----------
// Forge's `edit` writes the new lines and keeps them only when it reads back
// the model before with the one element added or gone. Otherwise its message
// is shown and the files are as they were.
function openAddForm(button) {
  const slot = $(`[data-addslot="${CSS.escape(button.dataset.add)}"]`, $("#view-model"));
  if (!slot) return;
  if (!slot.hidden) { closeAddForm(slot); return; }
  Object.assign(slot.dataset, { ns: button.dataset.ns, group: button.dataset.group, after: button.dataset.after, kind: button.dataset.kind === "column" ? "column" : "input" });
  const where = button.dataset.group || "the top level";
  const column = button.dataset.kind === "column";
  slot.innerHTML = `<div class="fedit">
    ${column ? `<p class="note">A new column of ${esc(where)}, calculated for each row.</p>` : `<div class="chips" role="group" aria-label="Kind"><button type="button" class="chip" data-akind="input" aria-pressed="true">Input</button><button type="button" class="chip" data-akind="formula" aria-pressed="false">Formula</button></div>`}
    <div><label class="lab" for="a-name-${esc(button.dataset.add)}">Name</label><input class="txt" id="a-name-${esc(button.dataset.add)}" data-aname autocomplete="off" autocapitalize="off" autocorrect="off" spellcheck="false" placeholder="new_name"></div>
    <div data-avalue ${column ? "hidden" : ""}><label class="lab" for="a-value-${esc(button.dataset.add)}">Value</label><input class="txt" id="a-value-${esc(button.dataset.add)}" data-ainput inputmode="decimal" autocomplete="off" placeholder="0"></div>
    <div data-aformula ${column ? "" : "hidden"}><label class="lab" for="a-formula-${esc(button.dataset.add)}">Formula</label><div class="suggest" data-suggest hidden aria-label="Suggestions"></div><input class="txt" id="a-formula-${esc(button.dataset.add)}" data-aform autocomplete="off" autocapitalize="off" autocorrect="off" spellcheck="false" placeholder="=…"></div>
    ${column ? "" : `<div><label class="lab" for="a-unit-${esc(button.dataset.add)}">Unit (optional)</label><input class="txt" id="a-unit-${esc(button.dataset.add)}" data-aunit autocomplete="off" autocapitalize="off" autocorrect="off" spellcheck="false" placeholder="CAD, %, months…"></div>`}
    <div class="btns"><button type="button" class="btn primary" data-asave>Check and add</button><button type="button" class="btn" data-acancel>Cancel</button></div>
    <div class="msg bad" data-amsg hidden></div></div>`;
  slot.hidden = false;
  $("[data-aname]", slot).focus();
}
function setAddKind(slot, kind) {
  slot.dataset.kind = kind;
  $$("[data-akind]", slot).forEach((b) => b.setAttribute("aria-pressed", b.dataset.akind === kind));
  $("[data-avalue]", slot).hidden = kind !== "input";
  $("[data-aformula]", slot).hidden = kind !== "formula";
}
function closeAddForm(slot) { slot.hidden = true; slot.innerHTML = ""; }

async function saveAdd(slot) {
  const msg = $("[data-amsg]", slot), button = $("[data-asave]", slot);
  const say = (text) => { msg.hidden = false; msg.textContent = text; };
  const { ns, group, after, kind } = slot.dataset;
  const name = $("[data-aname]", slot).value.trim();
  const prefix = `${ns ? "@" + ns + "." : ""}${group ? group + "." : ""}`;
  const path = prefix + name;
  if (formulaOf(S.structure, path) || allScalars().some((x) => x.path === path)) { say(`${path} is already in the model.`); return; }
  const unitField = $("[data-aunit]", slot), unit = unitField ? unitField.value.trim() || null : null;
  let entry;
  if (kind === "input") {
    const value = readNumber($("[data-ainput]", slot).value);
    if (value === null) { say("The value must be a number."); return; }
    entry = { kind, value, unit };
  } else {
    let formula = $("[data-aform]", slot).value.trim();
    if (!formula) { say("Write the formula."); return; }
    if (!formula.startsWith("=")) formula = "=" + formula;
    entry = { kind, formula, unit };
  }
  button.disabled = true;
  const why = await editModel([{ op: "add", path, ...(kind === "input" ? { value: entry.value } : { formula: entry.formula }), ...(entry.unit ? { unit: entry.unit } : {}), ...(!group && after ? { after } : {}) }]);
  if (why) {
    say(why);
    button.disabled = false;
  }
}

function askDelete(button) {
  const path = button.dataset.del;
  button.hidden = true;
  button.insertAdjacentHTML("afterend", `<span class="del-ask">Delete ${esc(leaf(path))}? <button type="button" class="edit-btn warn" data-delyes="${esc(path)}">Delete</button><button type="button" class="edit-btn" data-delno>Keep</button></span>`);
}
async function deleteElement(path) {
  const why = await editModel([{ op: "delete", path }]);
  if (why) {
    openSheet("Not deleted", `<p>${esc(why)}</p>`);
    renderModel(); updateValues();
  }
}

// ---------- grid view (roadmap S11) ----------
// The model as a spreadsheet reads it: each table as rows and columns, with
// the column's one formula in its header, and the scalars as a list. It is
// read-only and shows what Forge calculated (S.calc), formatted by the same
// rules as the cards; a tap on a cell or a header shows its value and
// formula, and takes you to its card, where editing stays.
const cellText = (v, path) => (typeof v === "number" ? fmtAt(v, path) : v === null || v === undefined ? "" : String(v));
function gridTable(t) {
  const calc = S.calc && S.calc.tables[t.name];
  const rows = calc ? Math.max(0, ...Object.values(calc.values).map((x) => x.length)) : 0;
  const labels = rowLabels(t.name);
  const head = t.columns.map((c) => `<th scope="col" class="g-head" data-gcol="${esc(t.name + "." + c.name)}"><span class="g-name">${esc(c.name)}</span>${c.unit ? `<span class="g-unit">${esc(c.unit)}</span>` : ""}${c.formula ? `<span class="g-formula" title="${esc(c.formula)}">${esc(c.formula)}</span>` : ""}</th>`).join("");
  let body = "";
  for (let i = 0; i < rows; i++) {
    body += `<tr><th scope="row">${esc(labels[i] ?? i + 1)}</th>` + t.columns.map((c) => {
      const v = calc.values[c.name] ? calc.values[c.name][i] : undefined;
      const path = `${t.name}.${c.name}`;
      const f = calc.formatted && calc.formatted[c.name] ? calc.formatted[c.name][i] : undefined;
      return `<td data-cell="${esc(path)}" data-row="${i}" data-value="${esc(v === undefined || v === null ? "" : String(v))}" class="${typeof v === "number" ? "num" : ""}${typeof v === "number" && v < 0 ? " neg" : ""}">${esc(f ?? cellText(v, path))}</td>`;
    }).join("") + `</tr>`;
  }
  if (!rows) body = `<tr><td class="empty" colspan="${t.columns.length + 1}">Not calculated</td></tr>`;
  return `<div class="eyebrow"><h2>${esc(t.name)}</h2><span class="tag">table · ${rows} rows</span></div>
    <div class="card grid-wrap"><table class="grid"><thead><tr><th class="corner" scope="col"></th>${head}</tr></thead><tbody>${body}</tbody></table></div>`;
}
function gridScalars(list) {
  let body = "", group;
  for (const s of list) {
    const g = split(s.path).group ?? "";
    if (g !== group) { group = g; body += `<tr class="g-group"><th colspan="2" scope="colgroup">${esc(g || "top level")}</th></tr>`; }
    const v = valueOf(s.path);
    body += `<tr data-gscalar="${esc(s.path)}" data-value="${esc(v === undefined || v === null ? "" : String(v))}"><th scope="row"><span class="g-name">${esc(leaf(s.path))}</span>${s.unit ? `<span class="g-unit">${esc(s.unit)}</span>` : ""}${s.formula != null ? `<span class="g-formula" title="${esc(s.formula)}">${esc(s.formula)}</span>` : ""}</th><td class="num${typeof v === "number" && v < 0 ? " neg" : ""}">${esc(S.calc ? formattedAt(s.path) ?? cellText(v, s.path) : "")}</td></tr>`;
  }
  return `<div class="eyebrow"><h2>scalars</h2><span class="tag">inputs and results</span></div>
    <div class="card grid-wrap flat"><table class="grid scalars"><thead><tr><th scope="col">name</th><th scope="col" class="num">value</th></tr></thead><tbody>${body}</tbody></table></div>`;
}
function renderGrid() {
  const v = $("#view-grid");
  if (!S.structure) {
    v.innerHTML = `<div class="msg bad">${esc(S.error || "Forge is starting…")}</div><p class="note">Fix it in the YAML tab, or open another model from Files.</p>`;
    return;
  }
  let h = `<p class="lead">The model as a spreadsheet reads it${S.scenario ? `, with the ${esc(S.scenario)} scenario applied` : ""}. Tap a cell or a heading for its formula; change things on the cards.</p>`;
  if (S.error) h += `<div class="msg bad">${esc(S.error)}</div>`;
  if (S.structure.scalars.length) h += gridScalars(S.structure.scalars);
  for (const t of S.structure.tables) h += gridTable(t);
  if (S.structure.includes.length) h += `<p class="note">Included files (${S.structure.includes.map((i) => esc(i.file)).join(", ")}) are on the Model tab.</p>`;
  v.innerHTML = h;
}
$("#view-grid").addEventListener("click", (e) => {
  const cell = e.target.closest("[data-cell]"), col = e.target.closest("[data-gcol]"), sc = e.target.closest("[data-gscalar]");
  const path = cell ? cell.dataset.cell : col ? col.dataset.gcol : sc ? sc.dataset.gscalar : null;
  if (!path) return;
  const entry = formulaOf(S.structure, path);
  const where = cell ? ` · row ${rowLabels(path.slice(0, path.lastIndexOf(".")))[Number(cell.dataset.row)] ?? Number(cell.dataset.row) + 1}` : "";
  const shown = cell ? cell.textContent : sc ? sc.querySelector("td").textContent : "";
  openSheet(`${path}${where}`,
    (cell || sc ? `<p class="res-val" data-gvalue>${esc(shown)}</p>` : "") +
    (entry && entry.formula ? `<p class="note">Formula</p><div class="formula" data-gformula>${esc(entry.formula)}</div>` : `<p class="note">${entry && entry.unit ? `${esc(entry.unit)} · ` : ""}typed in the file, not calculated</p>`) +
    `<div class="btns" style="margin-top:12px"><button type="button" class="btn primary" data-gocard="${esc(path)}">Show its card</button></div>`);
});
$("#sheet-body").addEventListener("click", (e) => {
  const go = e.target.closest("[data-gocard]");
  if (!go) return;
  $("#sheet").hidden = true;
  show("model");
  const card = $(`#view-model [data-card="${CSS.escape(go.dataset.gocard)}"]`);
  if (card) { card.scrollIntoView({ block: "center" }); card.classList.remove("pulse"); void card.offsetWidth; card.classList.add("pulse"); }
});

// ---------- YAML view ----------
function renderYaml() {
  const v = $("#view-yaml");
  if (!(S.yamlFile in S.files)) S.yamlFile = S.main;
  v.innerHTML = `
    <p class="lead">The YAML is the model. Every card comes from these files, and a change on a card is written back here, comments and all.</p>
    <div class="chips" role="group" aria-label="File">${Object.keys(S.files).map((n) => `<button type="button" class="chip" data-file="${esc(n)}" aria-pressed="${n === S.yamlFile}">${esc(n)}${n === S.main ? "<small>main</small>" : ""}</button>`).join("")}</div>
    <div class="suggest" data-suggest hidden aria-label="Keys of the model format"></div>
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
// On a wide screen the cards are beside the editor, so Forge reads the text
// as it is typed (after a pause); on a phone the edit is applied with a button.
let editing = 0;
vy.addEventListener("input", (e) => {
  if (e.target.id !== "yaml-editor") return;
  paintYaml();
  showKeySuggestions(e.target);
  if (!yamlDirty() || !S.wide) return;
  clearTimeout(editing);
  editing = setTimeout(applyEditor, 450);
});
vy.addEventListener("scroll", (e) => { if (e.target.id === "yaml-editor") paintYaml(); }, true);
vy.addEventListener("keydown", (e) => {
  if (e.target.id === "yaml-editor" && e.key === "Tab") { e.preventDefault(); document.execCommand("insertText", false, "  "); }
});
for (const type of ["keyup", "click", "focusin"]) {
  vy.addEventListener(type, (e) => { if (e.target.id === "yaml-editor") showKeySuggestions(e.target); });
}
vy.addEventListener("mousedown", (e) => { if (e.target.closest("[data-sug]")) e.preventDefault(); });
// On a phone the editor is taller than what is left above the keyboard, and
// the page scrolls to keep the caret in view, which takes the strip (above
// the editor) off the top of the screen. While the editor has the focus the
// strip is docked at the bottom of what is visible: on the keyboard.
const vv = window.visualViewport;
function placeDock() {
  const keyboard = vv ? Math.max(0, innerHeight - vv.height - vv.offsetTop) : 0;
  const tabs = $(".tabs");
  document.documentElement.style.setProperty("--dock", (keyboard || (tabs ? tabs.offsetHeight : 0)) + "px");
}
if (vv) { vv.addEventListener("resize", placeDock); vv.addEventListener("scroll", placeDock); }
let undock = 0;
vy.addEventListener("focusin", (e) => {
  if (e.target.id !== "yaml-editor" || S.wide) return;
  clearTimeout(undock);
  placeDock();
  $("#view-yaml [data-suggest]").classList.add("docked");
});
vy.addEventListener("focusout", (e) => {
  if (e.target.id !== "yaml-editor") return;
  // Not at once: a tap on a suggestion takes the focus before it is a click
  undock = setTimeout(() => { const strip = $("#view-yaml [data-suggest]"); if (strip) strip.classList.remove("docked"); }, 700);
});
vy.addEventListener("click", async (e) => {
  const sug = e.target.closest("[data-sug]");
  if (sug) { takeSuggestion(sug); return; }
  const f = e.target.closest("[data-file]");
  if (f) { S.yamlFile = f.dataset.file; renderYaml(); return; }
  if (e.target.id === "yaml-revert") { renderYaml(); return; }
  if (e.target.id === "yaml-apply") await applyEditor();
});
async function applyEditor() {
  const editor = $("#yaml-editor"), msg = $("#yaml-msg");
  if (!editor) return;
  const file = S.yamlFile, text = editor.value;
  const files = { ...S.files, [file]: text };
  try {
    await call("validate", { files, main: S.main, expectations: false });
    if (editor.value !== text || S.yamlFile !== file) return; // typed on meanwhile
    S.files = files; save();
    msg.className = "msg"; msg.textContent = "Forge read it. The cards are up to date.";
    $("#yaml-apply").disabled = $("#yaml-revert").disabled = true;
    await load();
  } catch (err) {
    if (editor.value !== text) return;
    msg.className = "msg bad"; msg.textContent = `Forge did not accept it: ${err.message}`;
  }
}
// A change on a card shows in the editor beside it, unless someone is typing there
function refreshEditor(file) {
  const editor = $("#yaml-editor");
  if (!editor || S.yamlFile !== file || document.activeElement === editor) return;
  editor.value = S.files[file];
  paintYaml();
}

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
  <div class="eyebrow" id="an-seek"><h2>goal seek</h2><span class="tag">forge goal-seek · break-even</span><span data-help-slot="goal-seek"></span></div>
  <div class="card row form">
    ${outs.length && ins.length ? `<div><label class="lab" for="sk-out">Result</label><select class="sel" id="sk-out">${outs.map((s) => opt(s.path, A.out)).join("")}</select></div>
    <div class="grid2"><div><label class="lab" for="sk-target">Target</label><input class="txt" id="sk-target" inputmode="decimal" value="${esc(A.target ?? "")}" placeholder="0 for break-even"></div>
    <div><label class="lab">Now</label><div class="now" id="sk-now"></div></div></div>
    <div><label class="lab" for="sk-vary">By changing</label><select class="sel" id="sk-vary">${ins.map((s) => opt(s.path, A.vary)).join("")}</select></div>
    <div class="btns"><button class="btn primary" type="button" id="sk-run">Find the value</button><button class="btn" type="button" id="be-run">Break-even</button></div>
    <div id="sk-result"></div>` : `<p class="note">Goal seek needs an input and a result in the model's scalars.</p>`}
  </div>
  <div class="eyebrow" id="an-scn"><h2>scenarios</h2><span class="tag">forge scenarios</span><span data-help-slot="scenarios"></span></div>
  <div class="card row" id="scn-box">${S.scenarios ? "" : S.scenarioError ? scenarioWhy(S.scenarioError) : `<p class="note">This model has no scenarios section.</p>`}</div>
  <div class="eyebrow" id="an-tor"><h2>tornado</h2><span class="tag">forge tornado</span><span data-help-slot="tornado"></span></div>
  <div class="card row"><div class="btns"><button class="btn" type="button" id="tor-run">Rank the inputs</button></div><div id="tor-box"></div></div>
  <div class="eyebrow" id="an-sim"><h2>simulate</h2><span class="tag">forge simulate · Latin hypercube</span><span data-help-slot="simulate"></span></div>
  <div class="card row form">
    <div class="grid2"><div><label class="lab" for="sim-n">Runs</label><select class="sel" id="sim-n">${[1000, 5000, 10000].map((n) => `<option ${n === (A.runs || 5000) ? "selected" : ""}>${n}</option>`).join("")}</select></div>
    <div><label class="lab" for="sim-seed">Seed</label><input class="txt" id="sim-seed" inputmode="numeric" value="${A.seed ?? 7}"></div></div>
    <div class="btns"><button class="btn primary" type="button" id="sim-run">Run the simulation</button></div><div id="sim-box"></div>
  </div>
  <div class="eyebrow" id="an-var"><h2>budget against actual</h2><span class="tag">forge variance</span><span data-help-slot="variance"></span></div>
  <div class="card row form">${A.others.length ? `<div class="grid2"><div><label class="lab" for="var-file">Actual</label><select class="sel" id="var-file">${A.others.map((f) => `<option ${f === A.actual ? "selected" : ""}>${esc(f)}</option>`).join("")}</select></div>
    <div><label class="lab" for="var-th">Alert above (%)</label><input class="txt" id="var-th" inputmode="decimal" value="${A.threshold ?? 10}"></div></div>
    <div class="btns"><button class="btn primary" type="button" id="var-run">Compare with ${esc(S.main)}</button></div><div id="var-box"></div>`
    : `<p class="note">Add a second model with the same names (the actuals) on the Files tab to compare it with this one.</p>`}</div>`;
  updateSeekNow();
  drawScenarios();
}
function updateSeekNow() {
  const el = $("#sk-now");
  if (el) el.textContent = fmtAt(valueOf(S.analyze.out), S.analyze.out);
}
function drawScenarios() {
  const box = $("#scn-box");
  if (!box || !S.scenarios) return;
  const out = S.analyze.out;
  const rows = S.scenarios.scenarios.map((s) => `<tr><th scope="row">${esc(s.name)}</th><td>${s.probability != null ? Math.round(s.probability * 100) + "%" : "—"}</td><td class="${s.outputs[out] < 0 ? "neg" : ""}">${esc(fmtAt(s.outputs[out], out))}</td></tr>`).join("");
  const ev = S.scenarios.expected_values ? S.scenarios.expected_values[out] : undefined;
  box.innerHTML = `<div class="scroll-x"><table class="report"><thead><tr><th scope="col">scenario</th><th scope="col">chance</th><th scope="col">${esc(out)}</th></tr></thead><tbody>${rows}</tbody>
    ${ev !== undefined ? `<tfoot><tr><th scope="row">expected</th><td></td><td>${esc(fmtAt(ev, out))}</td></tr></tfoot>` : ""}</table></div>
    <p class="note">The result chosen under goal seek, in each scenario, and weighted by the chances.</p>
    ${S.scenarios.unweighted && S.scenarios.unweighted.length ? `<p class="note" id="scn-unweighted">Not weighted, with no probability: ${esc(S.scenarios.unweighted.join(", "))}. They are choices to compare, not chances: tap one on the Model tab to apply it.</p>` : ""}`;
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
    const r = breakEven ? await call("break_even", request({ output: A.out, vary: A.vary, plan: true }))
      : await call("goal_seek", request({ target: A.out, value: target, vary: A.vary, plan: true }));
    A.solution = r.solution;
    box.innerHTML = `<div class="answer"><div class="note">${esc(r.vary)}</div><div class="res-val" data-value="${r.solution}">${esc(fmtAt(Number(r.solution.toFixed(6)), r.vary))}</div>
      <div class="note">gives ${esc(r.target)} = ${esc(fmtAt(r.achieved, r.target))}, against ${esc(fmtAt(valueOf(r.vary), r.vary))} now. ${r.converged ? `Found in ${r.iterations} calculations.` : "Forge did not converge."}</div></div>
      <div class="btns"><button class="btn" type="button" id="sk-use">Use this value</button></div>`;
  } catch (e) { fail(box, e); }
}
function drawTornado(t) {
  const box = $("#tor-box"), out = t.output;
  const lo = Math.min(...t.bars.map((b) => Math.min(b.output_at_low, b.output_at_high)), t.base_value);
  const hi = Math.max(...t.bars.map((b) => Math.max(b.output_at_low, b.output_at_high)), t.base_value);
  const pos = (v) => ((v - lo) / (hi - lo || 1)) * 100;
  box.innerHTML = `<p class="note" style="margin:10px 0">How far ${esc(t.output)} moves when each input goes from its low to its high value, one at a time. The line is the plan, ${esc(fmtAt(t.base_value, out))}. Copper means the result drops.</p><div class="tor">` +
    t.bars.map((b) => {
      const a = Math.min(b.output_at_low, b.output_at_high), z = Math.max(b.output_at_low, b.output_at_high);
      const down = b.output_at_high < b.output_at_low;
      return `<div class="tor-row"><div class="tor-name"><span>${esc(b.input_name)}</span><span>swing ${esc(shortAt(b.abs_swing, out))}</span></div>
        <div class="tor-track"><div class="tor-bar" style="left:${pos(a)}%;width:${Math.max(0.5, pos(z) - pos(a))}%;background:${down ? "var(--ember)" : "var(--accent)"}"></div><div class="tor-axis" style="left:${pos(t.base_value)}%"></div></div>
        <div class="tor-ends"><span>${esc(fmtAt(b.input_low, b.input_name))} → ${esc(shortAt(b.output_at_low, out))}</span><span>${esc(fmtAt(b.input_high, b.input_name))} → ${esc(shortAt(b.output_at_high, out))}</span></div></div>`;
    }).join("") + "</div>";
}
function drawSim() {
  const A = S.analyze, box = $("#sim-box"), r = A.sim;
  if (!r) return;
  const outs = r.output_order || Object.keys(r.outputs); // the model's order, not the keys sorted as text
  if (!outs.includes(A.simOut)) A.simOut = outs.find((o) => !/\(row \d+\)/.test(o)) || outs[0];
  const o = r.outputs[A.simOut], out = A.simOut;
  const th = Object.entries(o.thresholds || {});
  const p = o.percentiles;
  box.innerHTML = `<div style="margin-top:10px"><label class="lab" for="sim-out">Output</label><select class="sel" id="sim-out">${outs.map((x) => opt(x, A.simOut)).join("")}</select></div>
    <div class="stats" style="margin-top:10px">${[["mean", o.mean], ["P5", p.p5], ["P50", p.p50], ["P95", p.p95]].map(([k, v]) => `<div class="stat"><b>${k}</b><span>${esc(shortAt(v, out))}</span></div>`).join("")}</div>
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
        <div class="var-nums"><span>${esc(fmtAt(x.budget, x.name))} → ${esc(fmtAt(x.actual, x.name))}</span><span>${x.variance_pct == null ? "" : x.pct_meaningful === false ? "n/m" : (x.variance_pct > 0 ? "+" : "") + x.variance_pct.toFixed(1) + "%"}</span></div></div>`;
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
    try { drawTornado(await call("tornado", request({ plan: true }))); } catch (err) { fail(box, err); }
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

// ---------- Tools view: the MCP server's tools, run here (studio/tools.js) ----------
// The open model as the Tools view sends it
function toolModel() {
  const first = inputs()[0];
  return { A: defaults(), v: first ? first.value : 1, main: S.main, scenario: S.scenario, scenarios: S.structure.scenarios };
}
function renderTools() {
  const v = $("#view-mcp");
  if (!S.structure) { v.innerHTML = `<div class="msg bad">${esc(S.error || "No model")}</div>`; return; }
  v.innerHTML = toolsHtml(S.tools.open, S.tools.out, toolModel());
}
$("#view-mcp").addEventListener("click", async (e) => {
  const t = e.target.closest("[data-tool]");
  if (t) { S.tools.open = S.tools.open === t.dataset.tool ? null : t.dataset.tool; renderTools(); return; }
  const c = e.target.closest("[data-copy]");
  if (c) {
    const said = $("[data-copied]", c.parentElement);
    try { await navigator.clipboard.writeText(S.tools.answer); said.textContent = "Copied."; } catch (err) { said.textContent = "This browser did not allow copying: use Download."; }
    return;
  }
  const d = e.target.closest("[data-dl-answer]");
  if (d) { download(`forge_${d.dataset.dlAnswer}.json`, S.tools.answer, "application/json"); return; }
  const r = e.target.closest("[data-run]");
  if (r) {
    const name = r.dataset.run, box = $(`[data-out="${name}"]`), args = toolArgs(name, toolModel());
    if (args.files) args.files = S.files;
    busy(box, "Running…");
    try {
      // The whole answer, never cut (ADR-077)
      S.tools.answer = JSON.stringify(await call(name, args), null, 2);
      S.tools.out = [name, answerHtml(name, S.tools.answer)];
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
    <div class="eyebrow"><h2>save</h2><span class="tag">files, icloud drive</span></div>
    <div class="card row form"><div class="file"><div><b>${esc(modelFiles().join(" + "))}</b><p class="note">On an iPhone or iPad, Save opens the share sheet: choose Save to Files, then iCloud Drive and a folder. Elsewhere the files are downloaded.</p></div>
      <button class="btn primary" type="button" id="save-yaml">Save</button></div><div id="save-msg"></div></div>
    ${linkCard(modelFiles())}
    <div class="eyebrow"><h2>this model's files</h2><span class="tag">${Object.keys(S.files).length}</span></div>
    <div class="card stack">${Object.keys(S.files).map((f) => `<div class="row file"><div><b>${esc(f)}</b><p class="note">${f === S.main ? "Main model" : included.has(f) ? "Included by the main model" : "Another model"}${questionOf(f) ? `: ${esc(questionOf(f))} <span class="kind small" data-kind-of="${esc(f)}" data-kind="${esc(S.kinds[f] || "")}">${esc(S.kinds[f] || "")}</span>` : ""}</p></div>
      ${f !== S.main && !included.has(f) ? `<button class="btn" type="button" data-main="${esc(f)}">Open</button>` : ""}<button class="btn" type="button" data-dl="${esc(f)}">Download</button></div>`).join("")}</div>
    <div class="eyebrow"><h2>excel</h2><span class="tag">forge export</span></div>
    <div class="card row form"><div class="file"><div><b>${esc(S.main.replace(/\.ya?ml$/i, ""))}.xlsx</b><p class="note">The workbook <span class="mono">forge export</span> writes: every table and scalar group on its own sheet, with live formulas.</p></div>
      <button class="btn primary" type="button" id="xlsx" ${S.structure ? "" : "disabled"}>Save</button></div><div id="xlsx-msg"></div></div>
    ${S.calc && Object.keys(S.calc.tables).length ? `<div class="eyebrow"><h2>tables as CSV</h2><span class="tag">calculated</span></div><div class="card stack">${Object.keys(S.calc.tables).map((t) => `<div class="row file"><div><b>${esc(t)}.csv</b></div><button class="btn" type="button" data-csv="${esc(t)}">Download</button></div>`).join("")}</div>` : ""}
    <div class="eyebrow"><h2>your own model</h2></div>
    <div class="card row form"><label class="lab" for="imp">Open YAML files from this device (the main model and the files it includes), or an Excel workbook (.xlsx), which <span class="mono">forge import</span> turns into a model</label><input type="file" id="imp" accept=".yaml,.yml,.xlsx,text/yaml,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" multiple>
      <div id="imp-msg"></div><p class="note">They stay in this browser.</p></div>
    ${edited ? `<div class="btns"><button class="btn warn" type="button" id="reset">Undo all edits to ${esc(set.title)}</button></div>` : ""}
    ${welcomeSeen ? `<div class="btns"><button class="btn" type="button" id="welcome-again">Show the welcome tips again</button></div>` : ""}`;
  // Made now, so that a tap on Save can open the share sheet at once (below)
  if (S.structure) prepareWorkbook().catch(() => {});
  links.prepare().catch(() => {});
}
const vf = $("#view-files");
vf.addEventListener("click", async (e) => {
  const t = e.target;
  if (t.closest("[data-set]")) { openSet(t.closest("[data-set]").dataset.set); save(); show("model"); await load(); return; }
  if (t.dataset.main) { await openModel(t.dataset.main); return; }
  if (t.dataset.dl) download(t.dataset.dl, S.files[t.dataset.dl]);
  if (t.dataset.csv) download(t.dataset.csv + ".csv", csvOf(t.dataset.csv), "text/csv");
  if (t.id === "reset") { openSet(S.set); save(); show("model"); await load(); }
  if (t.id === "welcome-again") { setWelcome(false); renderModel(); updateValues(); show("model"); window.scrollTo(0, 0); return; }
  if (t.id === "xlsx") await exportWorkbook();
  if (t.id === "save-yaml") {
    const how = await saveFiles(modelFiles().map((f) => ({ name: f, data: S.files[f], type: "text/plain" })));
    $("#save-msg").innerHTML = how === "downloaded" ? `<p class="note" style="margin-top:8px">Downloaded ${esc(modelFiles().join(", "))}.</p>` : "";
  }
});

// ---------- saving: the share sheet where there is one ----------
// The main model and the files it includes: what another Forge needs to open it
function modelFiles() {
  const included = S.structure ? S.structure.includes.map((i) => i.file).filter((f) => f in S.files) : [];
  return [S.main, ...included];
}
// Safari opens the share sheet only within a tap: share() is called before
// anything is awaited. Where files cannot be shared (most desktop browsers),
// or the browser refuses, they are downloaded. Answers how they were saved.
async function saveFiles(list) {
  const files = list.map((f) => new File([f.data], f.name, { type: f.type }));
  if (navigator.canShare && navigator.canShare({ files })) {
    try {
      await navigator.share({ files });
      return "shared";
    } catch (e) {
      if (e.name === "AbortError") return "cancelled";
    }
  }
  for (const f of list) download(f.name, f.data, f.type);
  return "downloaded";
}

// ---------- Excel (roadmap S8): forge export and forge import, in the browser ----------
// A workbook crosses to and from forge-wasm as base64, as in the MCP tool (toBase64 and fromBase64, link.js)
const warningList = (title, warnings) => (warnings.length ? `<div class="msg" style="margin-top:8px"><b>${esc(title)}</b><ul>${warnings.map((w) => `<li>${esc(w)}</li>`).join("")}</ul></div>` : "");
const XLSX = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
// The workbook of the files as they are now, made once per change
let workbook = null;
async function prepareWorkbook() {
  const key = JSON.stringify([S.main, S.files]);
  if (workbook && workbook.key === key) return workbook;
  const r = await call("export", request());
  workbook = { key, name: `${S.main.replace(/\.ya?ml$/i, "")}.xlsx`, bytes: fromBase64(r.excel_base64), r };
  return workbook;
}
async function exportWorkbook() {
  const box = $("#xlsx-msg");
  const key = JSON.stringify([S.main, S.files]);
  try {
    // Ready: share within the tap. Not ready yet: make it, then share or download
    let w = workbook && workbook.key === key ? workbook : null;
    if (!w) { busy(box, "Forge is writing the workbook…"); w = await prepareWorkbook(); }
    const how = await saveFiles([{ name: w.name, data: w.bytes, type: XLSX }]);
    const { r } = w;
    box.innerHTML = (how === "cancelled" ? "" : `<p class="note" style="margin-top:8px">${esc(w.name)}: ${r.table_count} tables, ${r.scalar_count} scalars, ${(r.byte_count / 1024).toFixed(0)} KB${how === "downloaded" ? ", downloaded" : ""}.</p>`) + warningList("forge export says:", r.warnings);
  } catch (err) { fail(box, err); }
}
vf.addEventListener("change", async (e) => {
  if (e.target.id !== "imp" || !e.target.files.length) return;
  const workbook = [...e.target.files].find((f) => /\.xlsx$/i.test(f.name));
  if (workbook) {
    const box = $("#imp-msg");
    busy(box, "Forge is reading the workbook…");
    try {
      const name = workbook.name.replace(/\.xlsx$/i, ".yaml");
      const r = await call("import", { excel_base64: toBase64(new Uint8Array(await workbook.arrayBuffer())), name });
      Object.assign(S, { set: null, files: r.files, main: r.main, title: workbook.name });
      forget(); await shown();
      if (r.warnings.length) openSheet("Imported, with warnings", warningList("forge import says:", r.warnings));
    } catch (err) { fail(box, err); }
    return;
  }
  const files = {}, names = [...e.target.files].map((f) => f.name);
  for (const f of e.target.files) files[f.name] = await f.text();
  const main = names.find((n) => /^\s*_includes\s*:/m.test(files[n])) || names[0];
  Object.assign(S, { set: null, files, main, title: main });
  forget(); await shown();
});

// ---------- routing and layout ----------
// Below 900 px: one view at a time and the tabs at the bottom. Wider: the
// YAML editor on the left, the other views on the right, a divider between
// them that can be dragged, and controls to hide either side (ADR-056).
const VIEWS = ["model", "grid", "yaml", "analyze", "mcp", "files"];
// Each view drawn, then its help slots filled
const RENDER = Object.fromEntries(Object.entries({ model: () => { renderModel(); updateValues(); }, grid: renderGrid, yaml: renderYaml, analyze: renderAnalyze, mcp: renderTools, files: renderFiles })
  .map(([tab, draw]) => [tab, () => { draw(); fillHelp(); }]));
const WIDE = matchMedia("(min-width: 900px)");
const LAYOUT = "forge-studio-layout";
const layout = (() => { try { return JSON.parse(localStorage.getItem(LAYOUT)) || {}; } catch (e) { return {}; } })();
const keepLayout = () => { try { localStorage.setItem(LAYOUT, JSON.stringify(layout)); } catch (e) { /* private mode */ } };

function show(tab) {
  if (S.wide && tab === "yaml") tab = S.tab === "yaml" ? "model" : S.tab;
  S.tab = tab;
  for (const v of VIEWS) $("#view-" + v).hidden = !(v === tab || (S.wide && v === "yaml"));
  $$(".tab").forEach((b) => (b.dataset.tab === tab ? b.setAttribute("aria-current", "page") : b.removeAttribute("aria-current")));
  if (S.wide && layout.hide === "cards") hidePane(null);
  RENDER[tab]();
  if (S.wide) $("#view-" + tab).scrollTop = 0; else window.scrollTo(0, 0);
}
function renderAll() {
  RENDER[S.tab]();
  const editor = $("#yaml-editor");
  if (S.wide && editor && (!(S.yamlFile in S.files) || editor.value !== S.files[S.yamlFile]) && document.activeElement !== editor) renderYaml();
  updateStatus();
}
function arrange() {
  const was = S.wide;
  S.wide = WIDE.matches;
  document.body.classList.toggle("wide", S.wide);
  $("main").style.setProperty("--left", `${layout.left || 42}%`);
  hidePane(S.wide ? layout.hide || null : null, false);
  if (S.wide && !was) { renderYaml(); show(S.tab); }
  if (!S.wide && was) show(S.tab);
}
function hidePane(side, remember = true) {
  if (side) document.body.dataset.hide = side; else delete document.body.dataset.hide;
  $("#hide-editor").setAttribute("aria-pressed", side === "editor");
  $("#hide-cards").setAttribute("aria-pressed", side === "cards");
  if (remember) { layout.hide = side; keepLayout(); }
}
$$(".tab").forEach((b) => b.addEventListener("click", () => show(b.dataset.tab)));
document.addEventListener("click", (e) => { const to = e.target.closest("[data-show]"); if (to) { e.preventDefault(); show(to.dataset.show); const at = $(to.getAttribute("href")); if (at) at.scrollIntoView(); } });
$("#hide-editor").addEventListener("click", () => hidePane(document.body.dataset.hide === "editor" ? null : "editor"));
$("#hide-cards").addEventListener("click", () => hidePane(document.body.dataset.hide === "cards" ? null : "cards"));
WIDE.addEventListener("change", arrange);

// The divider: drag it, or focus it and use the arrow keys
const divider = $("#divider");
function setLeft(percent) {
  layout.left = Math.round(Math.min(75, Math.max(25, percent)) * 10) / 10;
  $("main").style.setProperty("--left", `${layout.left}%`);
  divider.setAttribute("aria-valuenow", Math.round(layout.left));
}
divider.addEventListener("pointerdown", (e) => {
  divider.setPointerCapture(e.pointerId);
  document.body.classList.add("dragging");
});
divider.addEventListener("pointermove", (e) => {
  if (!divider.hasPointerCapture(e.pointerId)) return;
  const box = $("main").getBoundingClientRect();
  setLeft(((e.clientX - box.left) / box.width) * 100);
});
divider.addEventListener("pointerup", (e) => {
  divider.releasePointerCapture(e.pointerId);
  document.body.classList.remove("dragging");
  keepLayout();
});
divider.addEventListener("keydown", (e) => {
  const step = { ArrowLeft: -2, ArrowRight: 2 }[e.key];
  if (!step) return;
  e.preventDefault();
  setLeft((layout.left || 42) + step);
  keepLayout();
});

// ---------- start ----------
let askedExample = null;
async function boot() {
  try {
    S.sets = await (await fetch("examples.json")).json();
  } catch (e) {
    S.error = "The examples could not be loaded."; S.sets = [];
  }
  try { HELP = await (await fetch("help.json")).json(); } catch (e) { /* no "?" then */ }
  try {
    useFormulaRules(await (await fetch("formula-rules.json")).json());
  } catch (e) { /* formulas stay plain in the editor; the cards do not need the colours */ }
  const params = new URLSearchParams(location.search), asked = params.get("example"), restored = restore();
  if (asked && restored && ownFiles().length) askedExample = [asked, params.get("model")]; // asks first (link.js)
  else if (asked || !restored) { if (S.sets.length) openSet(asked || S.sets[0].id, asked ? params.get("model") : null); }
  if (isModelLink(location.hash)) await links.open(location.hash, false);
  S.yamlFile = S.main;
  arrange();
  show("model");
  try {
    S.version = await start();
  } catch (e) {
    S.error = e.message; renderAll(); finish(); return;
  }
  templates = await call("templates", {}).catch(() => []);
  await load();
  if (askedExample) links.askExample(...askedExample);
  loadFunctions();
  keepOffline();
}

// Keeps the page on the device (sw.js) once Forge is running, so as not to
// compete with it for the connection; where a service worker is not allowed
// (plain http, a private window) Studio works as before, online.
function keepOffline() {
  if (!("serviceWorker" in navigator)) return;
  navigator.serviceWorker.register("sw.js").catch(() => {});
}
boot();
