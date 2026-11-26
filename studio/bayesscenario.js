// A Bayesian network in the scenario chosen on the Model view: the network's
// prior and tables may be formulas over the model's scalars (ADR-074), which
// a scenario moves, so forge_bayesian takes the scenario as the game and
// simulate tools do.

import { esc } from "./text.js";

// What forge_bayesian is asked besides the file: the evidence seen, and the
// scenario applied (none is the plan)
export const bayesExtra = (state) => ({
  evidence: Object.entries(state.evidence).map(([n, st]) => `${n}=${st}`),
  ...(state.scenario === null ? {} : { scenario: state.scenario }),
});

// A line above the chances that says which scenario they are drawn in
export const scenarioLine = (scenario) => (scenario === null ? "" : `<p class="note" data-bn-scenario="${esc(scenario)}">In the ${esc(scenario)} scenario.</p>`);
