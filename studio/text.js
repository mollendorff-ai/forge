// Text functions on YAML files. Nothing here reads a model's meaning: it
// finds a line and changes it, or colors the text for the editor. Forge reads
// the model (studio/forge.js).

export const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

// Change one input's value in a file's text and nothing else, keeping comments
// and layout. It handles the three ways an input is written: `name: 16`,
// `name: {value: 16, ...}` and a `value: 16` line under `name:`. `group` is
// the scalar's group (null at the top level). Answers null when the line is
// not found, and the caller says so instead of rewriting the file.
// The line of `name` (under the top-level `group`, or at the top level when
// group is null): its index, its indentation and the text after the colon.
function findKey(lines, group, name) {
  const keyRe = (k) => new RegExp(`^(\\s*)("${k}"|'${k}'|${k.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}):(.*)$`);
  let start = 0;
  if (group) {
    const g = lines.findIndex((l) => keyRe(group).test(l) && !/^\s/.test(l));
    if (g < 0) return null;
    start = g + 1;
  }
  for (let i = start; i < lines.length; i++) {
    const l = lines[i];
    if (group && /^\S/.test(l)) break;
    const m = keyRe(name).exec(l);
    if (!m || (group && m[1].length === 0) || (!group && m[1].length)) continue;
    return { i, indent: m[1].length, head: m[1] + m[2] + ":", rest: m[3] };
  }
  return null;
}

// The lines under the key at line i (more indented, or blank, or comments).
function* childLines(lines, i, indent) {
  for (let j = i + 1; j < lines.length; j++) {
    const lj = lines[j];
    if (lj.trim() && !lj.trim().startsWith("#") && lj.match(/^\s*/)[0].length <= indent) return;
    yield j;
  }
}

// A number as Forge reads it in a model (its YAML reader, checked on
// 2026-11-11): a sign, digits with or without a point, an exponent (1e-5,
// 1.5E+3), and 0x, 0o and 0b integers. 1_000, .inf and 1,000 are not numbers
// to Forge, so they are not here either.
const NUMBER = String.raw`[-+]?(?:0x[0-9a-fA-F]+|0o[0-7]+|0b[01]+|(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?)`;
// An input's value: a number, or a date (model format 5.4.0, ADR-060)
const VALUE = String.raw`${NUMBER}|\d{4}-\d{2}-\d{2}`;
const INLINE = new RegExp(String.raw`^(\s*)(${VALUE})(\s*(#.*)?)$`);
const FLOW = new RegExp(String.raw`^(\s*\{.*?\bvalue:\s*)(${VALUE}|null|~)(\s*[,}].*)$`);
const BLOCK = new RegExp(String.raw`^(\s+value:)(\s*)(${VALUE}|null|~)?(\s*(#.*)?)$`);

export function setInputText(text, group, name, v) {
  const lines = text.split("\n");
  const at = findKey(lines, group, name);
  if (!at) return null;
  const { i, head, rest } = at;
  const sm = INLINE.exec(rest);
  if (sm) { lines[i] = head + sm[1] + v + sm[3]; return lines.join("\n"); }
  const fm = FLOW.exec(rest);
  if (fm) { lines[i] = head + fm[1] + v + fm[3]; return lines.join("\n"); }
  for (const j of childLines(lines, i, at.indent)) {
    const vm = BLOCK.exec(lines[j]);
    if (vm) { lines[j] = vm[1] + (vm[2] || " ") + v + vm[4]; return lines.join("\n"); }
  }
  return null;
}

// A formula written for YAML in the style the file already uses for it:
// "double quotes", 'single quotes', or plain when that is safe; otherwise in
// double quotes.
function encodeFormula(formula, style, flow) {
  const plainSafe = !/: |\s#|^\s|\s$|["'`\\]/.test(formula) && !(flow && /[,{}[\]]/.test(formula));
  if (style === "plain" && plainSafe) return formula;
  if (style === "single") return `'${formula.replace(/'/g, "''")}'`;
  return `"${formula.replace(/\\/g, "\\\\").replace(/"/g, '\\"')}"`;
}
// Replace the YAML value at the start of `text` (after its spaces), keeping
// whatever follows it on the line: a comment, the rest of a flow mapping.
function replaceValue(text, formula, flow) {
  const lead = /^\s*/.exec(text)[0], v = text.slice(lead.length);
  let token, style;
  if (v.startsWith('"')) { token = /^"(?:[^"\\]|\\.)*"/.exec(v); style = "double"; }
  else if (v.startsWith("'")) { token = /^'(?:[^']|'')*'/.exec(v); style = "single"; }
  else { token = (flow ? /^[^,}\]]*?(?=\s*[,}\]]|\s+#|$)/ : /^.*?(?=\s+#|$)/).exec(v); style = "plain"; }
  if (!token || !token[0]) return null;
  return lead + encodeFormula(formula, style, flow) + v.slice(token[0].length);
}

// Change one formula in a file's text and nothing else (roadmap S7). It
// handles the ways a formula is written: `name: "=..."`, `name: '=...'`,
// `name: =...`, `name: {formula: "=...", ...}` and a `formula:` line under
// `name:`. Answers null when it cannot find the formula; the caller then
// asks Forge whether the new text holds exactly this change before keeping it.
export function setFormulaText(text, group, name, formula) {
  if (/[\r\n]/.test(formula)) return null;
  const lines = text.split("\n");
  const at = findKey(lines, group, name);
  if (!at) return null;
  const { i, head, rest } = at;
  const v = rest.trimStart();
  let changed = null;
  if (/^["']?=/.test(v)) changed = replaceValue(rest, formula, false);
  else if (v.startsWith("{")) {
    const m = /^(\s*\{(?:.*?[{,]\s*)?formula\s*:)(.*)$/.exec(rest);
    const tail = m && replaceValue(m[2], formula, true);
    if (tail != null) changed = m[1] + tail;
  } else if (!v || v.startsWith("#")) {
    for (const j of childLines(lines, i, at.indent)) {
      const fm = /^(\s+formula:)(.*)$/.exec(lines[j]);
      if (!fm) continue;
      const tail = replaceValue(fm[2], formula, false);
      if (tail == null) return null;
      lines[j] = fm[1] + tail;
      return lines.join("\n");
    }
  }
  if (changed == null) return null;
  lines[i] = head + changed;
  return lines.join("\n");
}

// ---------- adding and deleting an element (roadmap S9) ----------
// Both work on lines, as above, and answer null when the file is written in a
// way they do not handle (a group in a flow mapping, nested groups); the
// caller then points to the YAML tab. The caller asks Forge whether the new
// text holds exactly this change before keeping it.

// A name Studio writes as a plain YAML key; Forge decides whether it is a
// valid name for the model.
export const NAME = /^[A-Za-z_][A-Za-z0-9_]*$/;

// The last line of the block of the key at line i: its own line, the lines
// indented under it, and a block sequence written at its own indentation
// ("- x"). Blank lines and comments after the last of them are not part of it.
function blockEnd(lines, i, indent) {
  let end = i;
  for (let j = i + 1; j < lines.length; j++) {
    const t = lines[j].trim();
    if (!t || t.startsWith("#")) continue;
    const ind = lines[j].match(/^\s*/)[0].length;
    if (ind > indent || (ind === indent && /^-(\s|$)/.test(t))) { end = j; continue; }
    break;
  }
  return end;
}
// The indentation of the first key under the key at line i, or null.
function childIndent(lines, i, indent) {
  for (let j = i + 1; j < lines.length; j++) {
    const t = lines[j].trim();
    if (!t || t.startsWith("#")) continue;
    const ind = lines[j].match(/^\s*/)[0].length;
    return ind > indent ? ind : null;
  }
  return null;
}
// A unit as YAML: plain when it is a word or a few, otherwise in double quotes
const yamlText = (s) => (/^[A-Za-z][A-Za-z0-9_ ./-]*$/.test(s) && !/\s$/.test(s) ? s : JSON.stringify(s));

// Add an element at the end of `group` (a top-level key of the file), or, at
// the top level (group null), after the block of the top-level scalar
// `entry.after`. `entry` is one of
//   {kind: "input", value, unit}     name: 16, or value: and unit: under name:
//   {kind: "formula", formula, unit} name: with formula: (and unit:) under it
//   {kind: "column", formula}        name: "=..." in a table
// The new lines take the indentation of the keys already there.
export function addEntryText(text, group, name, entry) {
  if (!NAME.test(name)) return null;
  if (entry.formula != null && (/[\r\n]/.test(entry.formula) || !entry.formula.startsWith("="))) return null;
  if (entry.unit != null && /[\r\n]/.test(entry.unit)) return null;
  const lines = text.split("\n");
  if (findKey(lines, group, name)) return null;
  let at, pad;
  if (group) {
    const g = findKey(lines, null, group);
    if (!g || g.rest.trim() && !g.rest.trim().startsWith("#")) return null;
    const ind = childIndent(lines, g.i, 0);
    if (ind == null) return null;
    at = blockEnd(lines, g.i, 0) + 1;
    pad = " ".repeat(ind);
  } else {
    const a = entry.after && findKey(lines, null, entry.after);
    if (!a) return null;
    at = blockEnd(lines, a.i, 0) + 1;
    pad = "";
  }
  const inner = pad + (pad || "  ");
  const unit = entry.unit ? [`${inner}unit: ${yamlText(entry.unit)}`] : [];
  let add;
  if (entry.kind === "input") {
    if (typeof entry.value !== "number" || !Number.isFinite(entry.value)) return null;
    add = unit.length ? [`${pad}${name}:`, `${inner}value: ${entry.value}`, ...unit] : [`${pad}${name}: ${entry.value}`];
  } else if (entry.kind === "formula") {
    add = [`${pad}${name}:`, `${inner}formula: ${encodeFormula(entry.formula, "double")}`, ...unit];
  } else if (entry.kind === "column" && group) {
    add = [`${pad}${name}: ${encodeFormula(entry.formula, "double")}`];
  } else return null;
  lines.splice(at, 0, ...add);
  return lines.join("\n");
}

// Delete an element: its key line and the lines under it. A comment on the
// lines above it is kept (it may be about what follows). The last element of
// a group is not deleted here: the group would be left empty.
export function deleteEntryText(text, group, name) {
  const lines = text.split("\n");
  const at = findKey(lines, group, name);
  if (!at) return null;
  const end = blockEnd(lines, at.i, at.indent);
  if (group) {
    const g = findKey(lines, null, group);
    const gEnd = blockEnd(lines, g.i, 0);
    const others = lines.slice(g.i + 1, gEnd + 1).some((l, k) => {
      const j = g.i + 1 + k, t = l.trim();
      return (j < at.i || j > end) && t && !t.startsWith("#");
    });
    if (!others) return null;
  }
  lines.splice(at.i, end - at.i + 1);
  return lines.join("\n");
}


// Formula colours come from the VS Code extension's grammar,
// editors/vscode/syntaxes/forge-formula.injection.json (ADR-056 decision 4).
// build.mjs writes its patterns, in its order, to formula-rules.json; the page
// loads them with useFormulaRules(). Only two things are Studio's own: which
// CSS class each of the grammar's scopes gets, and the YAML around a formula
// (keys, comments, strings), which is a small line scanner below.
export const SCOPE_CLASS = {
  "keyword.operator.formula.forge": "t-eq",
  "string.quoted.double.formula.forge": "t-str",
  "support.function.forge": "t-fn",
  "variable.other.include.forge": "t-inc",
  "constant.language.boolean.forge": "t-const",
  "variable.other.forge": "t-name",
  "constant.numeric.forge": "t-num",
  "keyword.operator.forge": "t-op",
  "punctuation.separator.forge": "t-punct",
};

// TextMate patterns are Oniguruma's; JavaScript runs them but not `(?i:...)`
// (case-insensitive group), which is written out as [tT][rR]... here. Anything
// else JavaScript cannot read throws, and the test (tests/highlight.mjs) fails.
export function compileRule(rule) {
  const insensitive = (body) => body.replace(/\\.|\[(?:[^\]\\]|\\.)*\]|[A-Za-z]/g, (t) => (/^[A-Za-z]$/.test(t) ? `[${t.toLowerCase()}${t.toUpperCase()}]` : t));
  const source = rule.match.replace(/\(\?i:((?:[^()\\]|\\.)*)\)/g, (_, body) => `(?:${insensitive(body)})`);
  // sticky, so \b and look-behinds see the text before the match, as in a TextMate grammar
  return new RegExp(source, "yd");
}
let formulaRules = [];
export function useFormulaRules(list) {
  formulaRules = list.map((r) => ({ re: compileRule(r), scope: r.scope, captures: Object.entries(r.captures || {}).sort((a, b) => a[0] - b[0]) }));
}

const span = (cls, s) => (s ? `<span class="${cls}">${esc(s)}</span>` : "");
// Where no rule matches, a character is plain, and from there the next
// position is tried: the earliest match, and among those the first rule, as in TextMate.
function paintMatch(m, rule) {
  const name = (scope) => SCOPE_CLASS[scope] || "";
  if (rule.scope) return span(name(rule.scope), m[0]);
  let out = "", at = 0;
  for (const [n, scope] of rule.captures) {
    const [from, to] = m.indices[n].map((x) => x - m.index);
    out += esc(m[0].slice(at, from)) + span(name(scope), m[0].slice(from, to));
    at = to;
  }
  return out + esc(m[0].slice(at));
}
function hlFormula(src) {
  let out = span("t-eq", "="), i = 1;
  while (i < src.length) {
    let hit = null;
    for (const rule of formulaRules) {
      rule.re.lastIndex = i;
      const m = rule.re.exec(src);
      if (m && m[0]) { hit = [m, rule]; break; }
    }
    if (!hit) { out += esc(src[i]); i++; continue; }
    out += paintMatch(hit[0], hit[1]);
    i += hit[0][0].length;
  }
  return out;
}
function hlQuoted(s, q) {
  const body = s.slice(1, s.endsWith(q) && s.length > 1 ? -1 : undefined);
  const close = s.length > 1 && s.endsWith(q) ? q : "";
  if (body.startsWith("=")) return span("t-q", q) + `<span class="t-formula">${hlFormula(body)}</span>` + span("t-q", close);
  return span("t-ystr", s);
}
function hlValue(v) {
  let out = "", i = 0, inFlow = 0;
  while (i < v.length) {
    const rest = v.slice(i);
    let m;
    if ((m = /^\s+/.exec(rest))) { out += esc(m[0]); i += m[0].length; continue; }
    if (/^\s*#/.test(rest) && (i === 0 || /\s/.test(v[i - 1]))) { out += span("t-comment", rest); break; }
    if (rest[0] === '"') { m = /^"(?:[^"\\]|\\.)*"?/.exec(rest); out += hlQuoted(m[0], '"'); i += m[0].length; continue; }
    if (rest[0] === "'") { m = /^'(?:[^']|'')*'?/.exec(rest); out += hlQuoted(m[0], "'"); i += m[0].length; continue; }
    if ("{[".includes(rest[0])) { inFlow++; out += span("t-punct", rest[0]); i++; continue; }
    if ("}]".includes(rest[0])) { inFlow = Math.max(0, inFlow - 1); out += span("t-punct", rest[0]); i++; continue; }
    if (rest[0] === "," && inFlow) { out += span("t-punct", ","); i++; continue; }
    if (inFlow && (m = /^([A-Za-z_][\w.-]*)(\s*:)(?=\s|$)/.exec(rest))) { out += span("t-key", m[1]) + span("t-punct", m[2]); i += m[0].length; continue; }
    if (rest[0] === "=" && !inFlow) {
      const c = rest.search(/\s#/);
      const f = c < 0 ? rest : rest.slice(0, c);
      out += `<span class="t-formula">${hlFormula(f)}</span>`; i += f.length; continue;
    }
    m = inFlow ? /^[^,{}\[\]\s][^,{}\[\]]*?(?=\s*[,}\]]|\s+#|$)/.exec(rest) : /^.+?(?=\s+#|$)/.exec(rest);
    if (!m) { out += esc(rest[0]); i++; continue; }
    const tok = m[0].trim() ? m[0] : rest[0];
    const cls = /^-?(\d+\.?\d*|\.\d+)([eE][-+]?\d+)?$/.test(tok.trim()) ? "t-num" : /^(true|false|null|~)$/i.test(tok.trim()) ? "t-const" : /^\d{4}(-\d{1,2}){0,2}$/.test(tok.trim()) ? "t-num" : "t-ystr";
    out += span(cls, tok); i += tok.length;
  }
  return out;
}
export function highlightYaml(text) {
  return text.split("\n").map((line) => {
    if (/^\s*#/.test(line)) return span("t-comment", line);
    if (/^(---|\.\.\.)\s*$/.test(line)) return span("t-punct", line);
    const m = /^(\s*)(-\s+)?("[^"]*"|'[^']*'|[^\s"'#\-{[][^:#]*?|-[^\s:#][^:#]*?)(\s*:)(?=\s|$)(.*)$/.exec(line);
    if (m) {
      const key = m[3], special = /^_/.test(key) ? "t-meta" : "t-key";
      return esc(m[1]) + (m[2] ? span("t-punct", m[2]) : "") + span(special, key) + span("t-punct", m[4]) + hlValue(m[5]);
    }
    const d = /^(\s*)(-\s+)(.*)$/.exec(line);
    if (d) return esc(d[1]) + span("t-punct", d[2]) + hlValue(d[3]);
    return hlValue(line);
  }).join("\n") + "\n";
}

