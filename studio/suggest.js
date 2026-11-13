// Suggestions while a formula is typed (roadmap S10). Nothing here knows
// what a name or a function is: the names come from the model as Forge
// returns it (structure.scalars, tables and includes) and the functions from
// Forge's own list (the `functions` tool). The page adds no list of its own.

// A formula is written in the scope of the group or table it sits in, as Forge
// reads it: a name of the same group or table is bare, any other is written
// group.name, table.column, or @file.group.name for an included file.
const stripNs = (path, ns) => (ns ? path.slice(ns.length + 2) : path);

// Where a path is written: {ns, group, table}. A scalar belongs to its group,
// a column to its table; a path in an included file belongs to that file.
export function scopeOf(structure, path) {
  const ns = /^@([^.]+)\./.exec(path);
  const part = ns ? structure.includes.find((i) => i.as === ns[1]) : structure;
  const own = ns ? path.slice(ns[0].length) : path;
  if (part) {
    for (const t of part.tables) {
      const tname = ns ? t.name.slice(ns[0].length) : t.name;
      if (t.columns.some((c) => `${tname}.${c.name}` === own)) return { ns: ns ? ns[1] : null, group: null, table: tname };
    }
  }
  const dot = own.lastIndexOf(".");
  return { ns: ns ? ns[1] : null, group: dot < 0 ? null : own.slice(0, dot), table: null };
}

// The names a formula in `scope` can use, each with how it is written there
export function namesFor(structure, scope) {
  const out = [];
  const mine = scope.ns ? structure.includes.find((i) => i.as === scope.ns) : structure;
  if (!mine) return out;
  const own = (p) => stripNs(p, scope.ns);
  for (const s of mine.scalars) {
    const p = own(s.path), dot = p.lastIndexOf(".");
    const group = dot < 0 ? null : p.slice(0, dot);
    out.push({ text: p, bare: !scope.table && group === scope.group ? p.slice(dot + 1) : null, kind: "scalar", unit: s.unit });
  }
  for (const t of mine.tables) {
    const tname = scope.ns ? t.name.slice(scope.ns.length + 2) : t.name;
    for (const c of t.columns) out.push({ text: `${tname}.${c.name}`, bare: scope.table === tname ? c.name : null, kind: "column", unit: c.unit });
  }
  if (!scope.ns) {
    for (const inc of structure.includes) {
      for (const s of inc.scalars) out.push({ text: s.path, bare: null, kind: "included", unit: s.unit });
      for (const t of inc.tables) for (const c of t.columns) out.push({ text: `${t.name}.${c.name}`, bare: null, kind: "included", unit: c.unit });
    }
  }
  return out;
}

// Flatten the functions tool's answer: [{name, syntax, description}]
export const functionsOf = (answer) => answer.categories.flatMap((c) => c.functions.map((f) => ({ name: f.name, syntax: f.syntax, description: f.description })));

// The word being typed at `caret`: from the last character that cannot be in
// a name back, with its position. Null inside a "text", in a number, or when
// nothing is being typed.
export function tokenAt(text, caret) {
  const before = text.slice(0, caret);
  if ((before.match(/"/g) || []).length % 2) return null;
  const m = /[A-Za-z0-9_.@]*$/.exec(before);
  const token = m[0];
  if (!token || /^[0-9.]/.test(token)) return null;
  return { start: caret - token.length, end: caret, token };
}

const LIMIT = { names: 16, functions: 8 };
// The two words of the formula language that are values, not names or functions
const CONSTANTS = ["TRUE", "FALSE"];
const byLength = (a, b) => a.length - b.length || a.localeCompare(b);

// What to offer for the word at `caret`: names first (as written in this
// scope, then in full), then functions. Each is {label, insert, start, end,
// kind, title}; `insert` replaces the word.
export function suggest(index, text, caret) {
  const at = tokenAt(text, caret);
  if (!at) return [];
  const low = at.token.toLowerCase();
  const seen = new Set(), out = [];
  const names = [];
  for (const n of index.names) {
    if (n.bare && n.bare.toLowerCase().startsWith(low) && !seen.has(n.bare)) { seen.add(n.bare); names.push({ label: n.bare, insert: n.bare, kind: n.kind, title: n.unit ? `${n.text} · ${n.unit}` : n.text }); }
  }
  names.sort((a, b) => byLength(a.label, b.label));
  const full = [];
  for (const n of index.names) {
    if (n.text.toLowerCase().startsWith(low) && !seen.has(n.text)) { seen.add(n.text); full.push({ label: n.text, insert: n.text, kind: n.kind, title: n.unit ? `${n.text} · ${n.unit}` : n.text }); }
  }
  full.sort((a, b) => byLength(a.label, b.label));
  for (const n of [...names, ...full].slice(0, LIMIT.names)) out.push(n);
  if (!/[.@]/.test(at.token)) {
    for (const c of CONSTANTS) if (c.toLowerCase().startsWith(low) && !seen.has(c)) out.push({ label: c, insert: c, kind: "constant", title: "A logical value" });
    const next = text[at.end];
    const fns = index.functions.filter((f) => f.name.toLowerCase().startsWith(low)).sort((a, b) => byLength(a.name, b.name));
    for (const f of fns.slice(0, LIMIT.functions)) out.push({ label: `${f.name}(`, insert: f.name + (next === "(" ? "" : "("), kind: "function", title: `${f.syntax} · ${f.description}`, syntax: f.syntax, description: f.description });
  }
  return out.map((o) => ({ ...o, start: at.start, end: at.end }));
}

// The text and caret after taking suggestion `s`
export function apply(text, s) {
  const next = text.slice(0, s.start) + s.insert + text.slice(s.end);
  return { text: next, caret: s.start + s.insert.length };
}

// ---------- keys of the model format in the YAML editor (roadmap S12) ----------
// The keys come from the model format's JSON Schema as Forge returns it (the
// `schema` tool), not from a list in the page. The schema describes a model
// with references, alternatives and if/then/else; a key is offered when it is
// a property of any schema the position in the file can be read as.

// The schemas a node can be: itself and everything it refers to or combines
function expand(node, defs, out = [], seen = new Set()) {
  if (!node || typeof node !== "object" || seen.has(node)) return out;
  seen.add(node);
  out.push(node);
  if (node.$ref) {
    const name = String(node.$ref).replace(/^#\/definitions\//, "");
    expand(defs[name], defs, out, seen);
  }
  for (const k of ["allOf", "anyOf", "oneOf"]) for (const n of node[k] || []) expand(n, defs, out, seen);
  for (const k of ["then", "else"]) expand(node[k], defs, out, seen);
  return out;
}
// The names of the keys above a line, from the top, with "[]" for a list
// item: `lines` are the lines above it, `indent` where the line starts and
// `dashLen` the width of its "- " (0 when it is not at a list item)
function keysAbove(lines, indent, dashLen) {
  const dash = dashLen > 0;
  const chain = dash ? ["[]"] : [];
  // `limit`: a line is above us when it is indented less; `column`: where the
  // keys of the mapping we are in start
  let limit = indent, column = indent + dashLen;
  for (let j = lines.length - 1; j >= 0; j--) {
    const l = lines[j];
    if (!l.trim() || l.trim().startsWith("#")) continue;
    const lm = /^(\s*)(-\s+)?(?:"([^"]+)"|'([^']+)'|([^\s:#"'][^:#]*?))\s*:(?:\s|$)/.exec(l);
    const ind = l.match(/^\s*/)[0].length;
    const lcol = ind + (lm && lm[2] ? lm[2].length : 0);
    if (!dash && lm && lm[2] && lcol === column && ind < column) {
      // the first key of the list item we are in
      chain.unshift("[]");
      limit = ind; column = ind;
      continue;
    }
    if (ind < limit) {
      if (!lm) { if (/^\s*-\s/.test(l)) { limit = ind; continue; } return null; }
      chain.unshift(lm[3] ?? lm[4] ?? lm[5]);
      limit = ind; column = ind;
      if (ind === 0) break;
    }
  }
  return chain;
}
// Where a position in a file is, as the names of the keys above it, with "[]"
// for a list item; null when the caret is not at a key: after a colon, in a
// comment, or in a list that is not at the start of an item.
export function keyContext(text, caret) {
  const lines = text.slice(0, caret).split("\n");
  const here = lines.pop();
  const m = /^(\s*)(-\s+)?([A-Za-z_][A-Za-z0-9_]*)$/.exec(here);
  if (!m) return null;
  const chain = keysAbove(lines, m[1].length, m[2] ? m[2].length : 0);
  if (!chain) return null;
  return { chain, prefix: m[3], start: caret - m[3].length };
}
// The schema for a model: `answer` is the schema tool's text, parsed once
export function schemaOf(answer) {
  const root = typeof answer === "string" ? JSON.parse(answer) : answer;
  return { root, defs: root.definitions || {} };
}
const typeOf = (nodes) => {
  const types = new Set(nodes.flatMap((n) => [].concat(n.type || [])));
  return types.has("object") || types.has("array") ? "block" : "scalar";
};
// The schemas for the position named by `chain`
function nodesAt(schema, chain) {
  let nodes = expand(schema.root, schema.defs);
  for (const seg of chain) {
    const next = [];
    for (const n of nodes) {
      if (seg === "[]") { if (n.items) expand(n.items, schema.defs, next); continue; }
      if (n.properties && n.properties[seg]) expand(n.properties[seg], schema.defs, next);
      else if (n.additionalProperties && typeof n.additionalProperties === "object") expand(n.additionalProperties, schema.defs, next);
      if (n.patternProperties) for (const v of Object.values(n.patternProperties)) expand(v, schema.defs, next);
    }
    nodes = next;
  }
  return nodes;
}
// Every key the schema names anywhere (for the test)
export function allKeys(schema) {
  const keys = new Set(Object.keys(schema.root.properties || {}));
  for (const d of Object.values(schema.defs)) for (const n of expand(d, schema.defs)) Object.keys(n.properties || {}).forEach((k) => keys.add(k));
  return keys;
}
// What to offer for the key being typed at `caret`: {label, insert, start,
// end, kind: "key", title}
export function suggestKeys(schema, text, caret) {
  if (!schema) return [];
  const at = keyContext(text, caret);
  if (!at) return [];
  const nodes = nodesAt(schema, at.chain);
  const found = new Map();
  for (const n of nodes) {
    for (const [k, v] of Object.entries(n.properties || {})) {
      if (!k.toLowerCase().startsWith(at.prefix.toLowerCase()) || found.has(k)) continue;
      found.set(k, { description: v.description || (n.description || ""), type: typeOf(expand(v, schema.defs)) });
    }
  }
  const siblings = new Set(text.split("\n").map((l) => /^\s*(?:-\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*:/.exec(l)).filter(Boolean).map((m) => m[1]));
  return [...found.entries()]
    .sort((a, b) => byLength(a[0], b[0]))
    .slice(0, 12)
    .map(([k, v]) => ({ label: `${k}:`, insert: v.type === "block" ? `${k}:` : `${k}: `, start: at.start, end: caret, kind: "key", title: v.description, used: siblings.has(k) }));
}

// ---------- names and functions inside a formula typed in the YAML editor ----------
// A formula in a file is a quoted text that starts with "=" after a key
// (`formula: "=SUM(...)"`, or `revenue: "=dozens * ..."` in a table). While
// the caret is inside one, the strip offers what the cards offer: the names
// that formula can use, written as it writes them, and Forge's functions.

// Where the caret is, when it is inside a formula not yet closed: the keys
// above it, its own key, the formula typed so far and where that starts
export function formulaContext(text, caret) {
  const lines = text.slice(0, caret).split("\n");
  const here = lines.pop();
  const m = /^(\s*)(-\s+)?(?:"([^"]+)"|'([^']+)'|([^\s:#"'-][^:#"']*?))\s*:\s+(?:"(=(?:[^"\\]|\\.)*)|'(=(?:[^']|'')*)|(=[^#"']*))$/.exec(here);
  if (!m) return null;
  const chain = keysAbove(lines, m[1].length, m[2] ? m[2].length : 0);
  if (!chain) return null;
  const formula = m[6] ?? m[7] ?? m[8];
  return { chain, key: m[3] ?? m[4] ?? m[5], formula, start: caret - formula.length };
}
// The scope of the formula an entry holds, from where it is in its file: the
// entry is the last of the keys down to it (its own key, or the key above a
// `formula:`), and what holds it is a table or a group
export function scopeInYaml(structure, file, chain, key) {
  const keys = [...chain, ...(key === "formula" ? [] : [key])].filter((k) => k !== "[]");
  const holder = keys.length >= 2 ? keys[keys.length - 2] : null;
  const inc = structure.includes.find((i) => i.file === file);
  const part = inc || structure;
  const isTable = holder != null && part.tables.some((t) => (inc ? t.name.slice(inc.as.length + 2) : t.name) === holder);
  return { ns: inc ? inc.as : null, group: holder && !isTable ? holder : null, table: isTable ? holder : null };
}
// What to offer at `caret` when it is in a formula of `file`; null when it is
// not in one. `structure` is the model as Forge reads it, or null for a file
// that is not part of it (the functions are still offered).
export function suggestInFormula({ structure, functions, file }, text, caret) {
  const at = formulaContext(text, caret);
  if (!at) return null;
  const names = structure ? namesFor(structure, scopeInYaml(structure, file, at.chain, at.key)) : [];
  return suggest({ names, functions }, at.formula, at.formula.length).map((o) => ({ ...o, start: o.start + at.start, end: o.end + at.start }));
}
