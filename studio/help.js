// Help where it is needed (ADR-076): a "?" on each section heading opens a
// sheet with a few sentences, the terms the section uses, examples to open
// and a link to the manual on GitHub at the release that is running; and,
// while a formula or a key is typed, the syntax and description Forge gives
// for the first function or key offered. The text is studio/help.json; the
// descriptions are Forge's own (its function list and its schema).

import { esc } from "./text.js";

export const REPOSITORY = "https://github.com/mollendorff-ai/forge";

// A section's "?" (nothing when there is no help for it)
export const helpButton = (help, id) => (help && help[id]
  ? `<button type="button" class="qm" data-help="${esc(id)}" aria-label="About ${esc(help[id].title.toLowerCase())}">?</button>` : "");

// The manual's page at the release that is running: GitHub's tag v<version>
export const docLink = (doc, version) => `${REPOSITORY}/blob/v${version}/${doc}`;

// The sheet's body for one section
export function helpBody(entry, version) {
  return `<p>${esc(entry.text)}</p>` +
    (entry.terms ? `<ul class="terms">${entry.terms.map(([t, d]) => `<li><b>${esc(t)}</b>: ${esc(d)}</li>`).join("")}</ul>` : "") +
    (entry.examples ? `<p class="sub">open an example</p><div class="btns">${entry.examples.map(([set, file, label]) => `<button type="button" class="btn" data-example="${esc(set)}" data-example-model="${esc(file)}">${esc(label)}</button>`).join("")}</div>` : "") +
    `<p class="sub">in the manual</p><p><a class="doc" href="${esc(docLink(entry.doc, version))}" target="_blank" rel="noopener">${esc(entry.title)}, for Forge ${esc(version)} ↗</a></p>`;
}

// The suggestion strip: its buttons in a row, and under them what Forge says
// of the first function or key offered
export function suggestionStrip(list, button) {
  const first = list.find((x) => (x.kind === "function" || x.kind === "key") && x.title);
  return `<div class="sug-row">${list.map(button).join("")}</div>` +
    (first ? `<p class="sug-help" data-sug-help>${esc(first.title)}</p>` : "");
}
