// The analyses that are models of their own (ADR-076): a decision tree, a
// real option, a bootstrap and a Bayesian network. Each runs its tool when
// the model opens and is drawn from the tool's answer; the page adds no
// number of its own. Every number shown carries the JSON pointer of the
// answer it comes from, and the value in full, for the screen test
// (tests/analyses.mjs).

import { esc } from "./text.js";
import { fmt } from "./format.js";

// The analysis sections of a file (Forge's `model` answers them) that are
// drawn here, each with its tool and what it answers
export const ANALYSES = {
  decision_tree: { tool: "decision_tree", title: "decision tree", about: "Choices and chances in turn. Forge works back from the ends: the best choice at each step, and what it is worth on average." },
  real_options: { tool: "real_options", title: "real options", about: "What the right, not the duty, to wait, expand or abandon adds to a project, beside deciding now." },
  bootstrap: { tool: "bootstrap", title: "bootstrap", about: "How sure an estimate from the data alone can be: Forge draws from the data again and again, and reads the spread." },
  bayesian_network: { tool: "bayesian", title: "bayesian network", about: "Chances that depend on one another. Tap what you have seen, and Forge gives the chances of everything else." },
};

// A number of the answer at `pointer`, as shown and in full
const num = (answer, pointer, shown = fmt) => {
  const v = pointer.split("/").slice(1).reduce((o, k) => (o == null ? o : o[k.replace(/~1/g, "/").replace(/~0/g, "~")]), answer);
  return `<span class="num" data-num="${esc(pointer)}" data-value="${esc(String(v))}">${esc(shown(v))}</span>`;
};
const key = (k) => String(k).replace(/~/g, "~0").replace(/\//g, "~1");
const pct = (v) => `${(v * 100).toFixed(1)}%`;

function decisionTree(r) {
  const root = r.node_results.root;
  const branches = Object.keys(root.branch_values);
  return `<p class="lead-num"><b>${esc(root.optimal_choice)}</b>, worth ${num(r, "/root_expected_value")} on average</p>
    <table class="report"><thead><tr><th scope="col">${esc(root.name)}</th><th scope="col">expected value</th></tr></thead><tbody>${branches.map((b) =>
      `<tr${b === root.optimal_choice ? ' class="best"' : ""}><th scope="row">${esc(b)}${b === root.optimal_choice ? ' <span class="pill live">best</span>' : ""}</th><td>${num(r, `/node_results/root/branch_values/${key(b)}`)}</td></tr>`).join("")}</tbody></table>
    <p class="sub">the best plan</p><ol class="path">${r.optimal_path.map((step) => `<li>${esc(step.trim())}</li>`).join("")}</ol>
    <p class="sub">what can happen on it</p><table class="report"><tbody>${r.risk_profile.outcomes.map((o, i) =>
      `<tr><th scope="row">${esc(o.path.slice(1).map((p) => p.split(".")[1]).join(", "))}</th><td>${num(r, `/risk_profile/outcomes/${i}/probability`, pct)}</td><td>${num(r, `/risk_profile/outcomes/${i}/value`)}</td></tr>`).join("")}</tbody></table>
    <p class="note">From ${num(r, "/risk_profile/worst_case")} to ${num(r, "/risk_profile/best_case")}; more than 0 with a chance of ${num(r, "/risk_profile/probability_positive", pct)}.</p>`;
}

function realOptions(r) {
  const names = Object.keys(r.options);
  return `<p class="lead-num">${num(r, "/project_value_with_options")} <span class="note">with the best option, against ${num(r, "/traditional_npv")} deciding now</span></p>
    <div class="options">${names.map((n) =>
      `<div class="option${n === r.best_option ? " best" : ""}"><div class="option-top"><b>${esc(n)}</b><span class="pill${n === r.best_option ? " live" : ""}">${esc(r.options[n].option_type)}</span></div>
        <div class="note">adds ${num(r, `/options/${key(n)}/value_added`)}; exercise when ${esc(r.options[n].optimal_trigger.replace(/^value /, "the value is "))}</div></div>`).join("")}</div>
    <p class="note">${esc(r.recommendation)}</p>`;
}

function bootstrap(r) {
  return `<p class="lead-num">${num(r, "/original_estimate")} <span class="note">from the data, and from ${num(r, "/iterations", (v) => v.toLocaleString("en-US"))} draws:</span></p>
    <table class="report"><thead><tr><th scope="col">sure to</th><th scope="col">from</th><th scope="col">to</th></tr></thead><tbody>${r.confidence_intervals.map((c, i) =>
      `<tr><th scope="row">${num(r, `/confidence_intervals/${i}/level`, (v) => `${Math.round(v * 100)}%`)}</th><td>${num(r, `/confidence_intervals/${i}/lower`)}</td><td>${num(r, `/confidence_intervals/${i}/upper`)}</td></tr>`).join("")}</tbody></table>
    <p class="note">Standard error ${num(r, "/bootstrap_std_error")}, bias ${num(r, "/bias")} (${esc(r.method)} method).</p>`;
}

// `evidence`: what the person has seen, {node: state}; a node seen is shown
// with its state pressed and no chances
function bayesian(r, nodes, evidence) {
  return `<p class="note">${Object.keys(evidence).length ? "Given what you have seen:" : "Nothing seen yet. Tap a state you have seen."}</p>` +
    nodes.map(({ name, states }) => {
      const q = r.queries[name], seen = evidence[name];
      return `<div class="bn-node" data-node="${esc(name)}"><p class="sub">${esc(name)}</p>${states.map((s, i) => {
        const pressed = seen === s;
        return `<div class="bn-state"><button type="button" class="chip" data-ev="${esc(name)}" data-state="${esc(s)}" aria-pressed="${pressed}">${esc(s)}</button>${q ? `<span class="bn-bar"><i style="width:${(q.probabilities[i] * 100).toFixed(2)}%"></i></span>${num(r, `/queries/${key(name)}/probabilities/${i}`, pct)}` : pressed ? '<span class="note">seen</span>' : ""}</div>`;
      }).join("")}</div>`;
    }).join("");
}

// A card's slot, before the answer comes
export const analysisSlot = (section) => `<div class="eyebrow"><h2>${esc(ANALYSES[section].title)}</h2><span class="tag">forge ${esc(ANALYSES[section].tool.replace("_", "-"))}</span></div>
  <div class="card row analysis" data-analysis="${esc(section)}"><p class="note">${esc(ANALYSES[section].about)}</p><div data-answer><p class="note">Forge is working it out…</p></div></div>`;

// The answer drawn into its slot; `nodes` and `evidence` for a network
export function drawAnalysis(slot, section, answer, nodes, evidence) {
  const box = slot.querySelector("[data-answer]");
  if (answer.error) { box.innerHTML = `<div class="msg bad">${esc(answer.error)}</div>`; return; }
  box.innerHTML = section === "decision_tree" ? decisionTree(answer)
    : section === "real_options" ? realOptions(answer)
      : section === "bootstrap" ? bootstrap(answer)
        : bayesian(answer, nodes, evidence);
}
