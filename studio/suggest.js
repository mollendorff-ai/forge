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
