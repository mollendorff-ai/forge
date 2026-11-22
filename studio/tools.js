// The Tools view: the MCP server's tools, run in the page on the open model
// with the same arguments (the files in place of paths) and the same answers.
// What each tool is sent and how the view looks; app.js runs them.

import { esc } from "./text.js";

export const TOOLS = [
  ["Core", [["validate", "Check a model for formula errors, circular references and type mismatches, and that it gives what its expect: states, in the file and each scenario."], ["calculate", "Calculate every formula, optionally under a named scenario."], ["audit", "Follow one variable's formula down to the inputs it reads."], ["report", "Groups with the same keys, side by side."]]],
  ["What-if", [["goal_seek", "Find the input value that makes an output hit a target."], ["break_even", "Find where an output reaches zero."], ["scenarios", "Run each probability-weighted scenario."], ["compare", "Put scenario results side by side."], ["sensitivity", "Vary one input over a range and watch an output."], ["tornado", "Rank inputs by how much they move one output."], ["variance", "Budget against actual, with favourable and unfavourable flags."]]],
  ["Uncertainty", [["simulate", "Monte Carlo with Latin hypercube sampling and a seed."], ["probability_seek", "Find the input that gives an output a chance of passing a threshold."], ["bootstrap", "Bootstrap confidence intervals from data."], ["decision_tree", "Backward induction: best path and expected value."], ["real_options", "Value the option to defer, expand or abandon."], ["bayesian", "Posterior probabilities in a Bayesian network."]]],
  ["Games", [["game", "Dominated strategies, equilibria and tournaments of the model's games."]]],
  ["Reference", [["functions", "The Excel-compatible functions, by category."], ["examples", "Runnable example models, by name."], ["schema", "The JSON Schema for Forge model files."]]],
];

// The request a tool is shown with and run with. `m`: the open model (main,
// scenario, scenarios), Analyze's choices (A: out, vary, target, actual) and
// the first input's value (v); `files` is shown as "…" and filled in to run
export function toolArgs(tool, m) {
  const { A, v } = m, base = { files: "…", main: m.main };
  switch (tool) {
    case "functions": case "examples": case "schema": return {};
    case "calculate": case "game": return { ...base, scenario: m.scenario };
    case "audit": return { ...base, variable: A.out };
    case "compare": return { ...base, scenarios: m.scenarios.slice(0, 2) };
    case "goal_seek": return { ...base, target: A.out, value: A.target ?? 0, vary: A.vary };
    case "break_even": return { ...base, output: A.out, vary: A.vary };
    case "sensitivity": return { ...base, vary: A.vary, range: [v * 0.8, v * 1.2, v * 0.1].map((x) => Number(x.toPrecision(4))).join(","), output: A.out };
    case "simulate": case "bootstrap": return { ...base, iterations: 1000, seed: 7 };
    case "probability_seek": return { ...base, goal: { output: A.out, threshold: "> 0", probability: 0.8 }, input: A.vary, min: Number((v * 0.5).toPrecision(4)), max: Number((v * 1.5).toPrecision(4)), iterations: 1000, seed: 7 };
    case "variance": return { files: "…", budget: m.main, actual: A.actual };
    default: return base;
  }
}

// The view: each tool, and the open one with its request, Run and its answer
export function toolsHtml(open, out, m) {
  return `<p class="lead">Forge's MCP server gives an AI agent these tools. Here they run in this page, on the open model, with the same arguments (the files in place of paths) and the same answers.</p>` +
    TOOLS.map(([group, list]) => `<div class="eyebrow"><h2>${esc(group)}</h2><span class="tag">${list.length} tools</span></div><div class="card stack">${list.map(([name, desc]) => `
      <div class="row" style="padding:0"><button type="button" class="tool" data-tool="${name}" aria-expanded="${open === name}"><span class="tool-name">forge_${name}</span><span class="pill live">runs here</span><span class="tool-desc">${esc(desc)}</span></button>
      ${open === name ? `<div class="tool-body"><p class="sub">request</p><pre class="code">${esc(`call("${name}", ${JSON.stringify(toolArgs(name, m), null, 2)})`)}</pre>
        <div class="btns"><button type="button" class="btn primary" data-run="${name}">Run</button></div><div data-out="${name}">${out && out[0] === name ? out[1] : ""}</div></div>` : ""}</div>`).join("")}</div>`).join("");
}

// An answer, whole (ADR-077): the box scrolls, and Copy and Download take all of it
export const answerHtml = (name, text) => `<p class="sub">answer</p><pre class="code">${esc(text)}</pre>
  <div class="btns"><button type="button" class="btn" data-copy="${name}">Copy</button><button type="button" class="btn" data-dl-answer="${name}">Download</button><span class="note" data-copied></span></div>`;
