// Forge Studio's "add a game" (ADR-076 decision 6): the templates Forge serves
// (the `templates` tool), each added by `edit`'s `insert`. The page writes no
// YAML and holds no template of its own; Forge's refusal is shown as it words it.

import { esc } from "./text.js";

// The card, from the answer of the `templates` tool
export function templateCard(list) {
  if (!list.length) return "";
  return `<div class="card row game-add" data-templates>
    <p class="sub">add a game</p>
    <div class="slide-row">
      <select class="sel" data-tsel aria-label="Template">${list.map((t) => `<option value="${esc(t.name)}">${esc(t.name.replace(/_/g, " "))}</option>`).join("")}</select>
      <input class="txt" type="text" data-tname aria-label="Name of the game" placeholder="${esc(list[0].default)}" autocapitalize="none" autocomplete="off" spellcheck="false">
      <button type="button" class="edit-btn" data-tadd>Add game</button>
    </div>
    <p class="note" data-tline>${esc(list[0].line)}</p>
    <div class="msg bad" data-tmsg hidden></div>
  </div>`;
}

// The change a card asks for: the template, and a name when one is typed
export function insertOf(card) {
  const name = card.querySelector("[data-tname]").value.trim();
  return { op: "insert", path: card.querySelector("[data-tsel]").value, ...(name ? { new_name: name } : {}) };
}

// A template chosen shows what it is and the name it would get; Add game
// asks `edit(changes)`, which answers why not, or null
export function listenTemplates(view, list, edit) {
  view.addEventListener("change", (e) => {
    const sel = e.target.closest && e.target.closest("[data-tsel]");
    if (!sel) return;
    const card = sel.closest("[data-templates]"), t = list().find((x) => x.name === sel.value);
    card.querySelector("[data-tline]").textContent = t.line;
    card.querySelector("[data-tname]").placeholder = t.default;
  });
  view.addEventListener("click", async (e) => {
    const add = e.target.closest("[data-tadd]");
    if (!add) return;
    const card = add.closest("[data-templates]"), msg = card.querySelector("[data-tmsg]");
    add.disabled = true;
    const why = await edit([insertOf(card)]);
    if (why) { msg.hidden = false; msg.textContent = why; add.disabled = false; }
  });
}
