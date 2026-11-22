// A set's models (ADR-077): the welcome card lists them as questions, and a
// strip at the top of the Model view opens any of them. The questions are
// the set's own (studio/build.mjs); the kind of each model is what Forge's
// `model` answers about its file, never read from the YAML here.

import { esc } from "./text.js";

// The kind of a model, by Forge's `model` answer: its games, else its first
// analysis section that is a kind of model of its own, else a calculation
// (a tornado is an analysis of a calculation, not another kind)
const KINDS = [["monte_carlo", "simulation"], ["decision_tree", "decision tree"], ["real_options", "real option"], ["bootstrap", "bootstrap"], ["bayesian_network", "Bayes"]];
export function kindOf(structure) {
  if (structure.games.length) return "game";
  const kind = KINDS.find(([section]) => structure.analyses.includes(section));
  return kind ? kind[1] : "calculation";
}

// A kind's badge; empty until Forge has answered for that file
const badge = (file, kinds) => `<span class="kind" data-kind-of="${esc(file)}" data-kind="${esc(kinds[file] || "")}">${esc(kinds[file] || "")}</span>`;

// The welcome card of a set of several models: what the page is, the set's
// questions (a tap opens that model), one tip, and Got it
export function questionsCard(title, models, current, kinds) {
  return `<section class="welcome card" id="welcome" aria-labelledby="welcome-title">
    <h2 id="welcome-title">${esc(title)}, in ${models.length} questions</h2>
    <p>Real models, calculated by Forge in your browser. Nothing you type leaves this device. Each question is a model of its own: tap one.</p>
    <div class="questions">${models.map(([file, , question]) => `<button type="button" class="question" data-model="${esc(file)}"${file === current ? ' aria-current="true"' : ""}>${badge(file, kinds)}<b>${esc(question)}</b>${file === current ? '<span class="pill live">open</span>' : '<span class="go" aria-hidden="true">›</span>'}</button>`).join("")}</div>
    <p class="tip">Drag a slider or type a number, and every result is calculated again. Tap Edit on a result to see its formula.</p>
    <button type="button" class="btn primary" id="welcome-done">Got it</button>
  </section>`;
}

// The strip: the set's models by their short names, the open one pressed
export function setStrip(models, current, kinds) {
  return `<div class="chips set-strip" role="group" aria-label="This set's models">${models.map(([file, name, question]) => `<button type="button" class="chip" data-model="${esc(file)}" aria-pressed="${file === current}" title="${esc(question)}">${esc(name)}${badge(file, kinds).replace('class="kind"', 'class="kind small"')}</button>`).join("")}</div>`;
}

// Fill in the badges once Forge has answered
export function drawKinds(root, kinds) {
  for (const el of root.querySelectorAll("[data-kind-of]")) {
    const kind = kinds[el.dataset.kindOf] || "";
    el.textContent = kind;
    el.dataset.kind = kind;
  }
}
