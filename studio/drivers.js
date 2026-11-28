// The "what drives it" list of a simulation's output. Each driver is an input
// and its rank correlation with the output; an input that is in a
// monte_carlo.correlations entry also carries correlated_with, the other
// inputs and the coefficients, because its rank correlation can come through
// the input it is correlated with and not from its own effect. Forge says
// which; this draws what forge simulate answers.

import { esc } from "./text.js";

// A driver's note, "correlated 0.5 with assumptions.winter_lift", or none
export const correlationNote = (d) => (d.correlated_with || []).map((c) => `correlated ${c.coefficient} with ${c.input}`).join(", ");

// The list, or nothing when the output has no drivers
export const driversHtml = (drivers) => (drivers && drivers.length
  ? `<p class="sub" style="margin-top:12px">what drives it</p>${drivers.map((d) => `<div class="var-nums" data-driver="${esc(d.input)}"><span class="mono">${esc(d.input)}</span><span>${d.rank_correlation.toFixed(2)}</span>${correlationNote(d) ? `<span class="note" data-correlated="${esc(correlationNote(d))}">${esc(correlationNote(d))}</span>` : ""}</div>`).join("")}`
  : "");
