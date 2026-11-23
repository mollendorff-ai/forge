// Forge Studio's game card edits (ADR-076 decision 5): rename a game, a player
// or a strategy, and add or delete a strategy or a player. The page writes no
// YAML: each button is one change of forge-wasm's `edit` (ADR-075), kept only
// when Forge accepts it, and Forge's refusal is shown as it words it.

import { esc } from "./text.js";

// The nine habits a tournament's player can play (the schema's `plays`)
export const HABITS = ["always_cooperate", "always_defect", "tit_for_tat", "suspicious_tit_for_tat", "tit_for_two_tats", "grim_trigger", "win_stay_lose_shift", "generous_tit_for_tat", "random"];

const option = (value, label) => `<option value="${esc(value)}">${esc(label)}</option>`;

// A strategy every player has is one path for all; one only some players have
// is named with its player's
function strategyPaths(r) {
  const players = r.players, all = new Set(players.flatMap((p) => r.strategies[p]));
  return [...all].flatMap((s) => players.every((p) => r.strategies[p].includes(s))
    ? [{ path: `${r.game}.strategies.${s}`, label: s }]
    : players.filter((p) => r.strategies[p].includes(s)).map((p) => ({ path: `${r.game}.strategies.${p}.${s}`, label: `${p}: ${s}` })));
}

// What a card can change, from Forge's answer for the game
function targets(r) {
  const tournament = r.kind === "tournament";
  const players = tournament ? r.ranking.map((s) => s.player) : r.players;
  const strategies = tournament ? [] : strategyPaths(r);
  return { tournament, players, strategies };
}

// The part of a game's card that changes it
export function gameEditor(r, open) {
  const { tournament, players, strategies } = targets(r);
  const g = r.game;
  const renames = [option(g, `the game, ${g}`), ...players.map((p) => option(`${g}.players.${p}`, `player ${p}`)), ...strategies.map((s) => option(s.path, `strategy ${s.label}`))].join("");
  const row = (name, body) => `<div class="slide-row" data-grow="${name}">${body}</div>`;
  const text = (label) => `<input class="txt" type="text" data-gname aria-label="${label}" placeholder="${label}" autocapitalize="none" autocomplete="off" spellcheck="false">`;
  const button = (op, label) => `<button type="button" class="edit-btn" data-gop="${op}">${label}</button>`;
  const pick = (label, list) => `<select class="sel" data-gsel aria-label="${label}">${list}</select>`;
  return `<details class="game-edit" data-gedit="${esc(g)}"${open ? " open" : ""}><summary>change this game</summary>` +
    row("rename", `${pick("What to rename", renames)}${text("new name")}${button("rename", "Rename")}`) +
    (tournament ? "" : row("add_strategy", `${text("new strategy")}${button("add_strategy", "Add strategy")}`) +
      row("delete_strategy", `${pick("Strategy to delete", strategies.map((s) => option(s.path, s.label)).join(""))}${button("delete_strategy", "Delete strategy")}`)) +
    row("add_player", `${text("new player")}${tournament ? `<select class="sel" data-gplays aria-label="What it plays">${HABITS.map((h) => option(h, h.replace(/_/g, " "))).join("")}</select>` : ""}${button("add_player", "Add player")}`) +
    row("delete_player", `${pick("Player to delete", players.map((p) => option(`${g}.players.${p}`, p)).join(""))}${button("delete_player", "Delete player")}`) +
    `<div class="msg bad" data-gmsg hidden></div></details>`;
}

// The change a row asks for
export function changeOf(game, op, row) {
  const name = (row.querySelector("[data-gname]")?.value ?? "").trim();
  const chosen = row.querySelector("[data-gsel]")?.value;
  switch (op) {
    case "rename": return { op, path: chosen, new_name: name };
    case "add_strategy": return { op, path: `${game}.strategies.${name}` };
    case "add_player": {
      const plays = row.querySelector("[data-gplays]")?.value;
      return { op, path: `${game}.players.${name}`, ...(plays ? { plays } : {}) };
    }
    default: return { op, path: chosen };
  }
}

// A button pressed: `edit(changes)` answers why not, or null; why not is shown
// in the card, as Forge words it
export function listenGameEdits(view, edit) {
  view.addEventListener("click", async (e) => {
    const button = e.target.closest("[data-gop]");
    if (!button) return;
    const box = button.closest("[data-gedit]"), msg = box.querySelector("[data-gmsg]");
    button.disabled = true;
    const why = await edit([changeOf(box.dataset.gedit, button.dataset.gop, button.closest("[data-grow]"))]);
    if (why) { msg.hidden = false; msg.textContent = why; button.disabled = false; }
  });
}
