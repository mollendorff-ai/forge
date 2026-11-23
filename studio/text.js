// Colours for the editor, and the one escape the page uses for text it writes
// into HTML. Nothing here reads a model's meaning, and nothing here changes a
// file: Forge writes the model's text (ADR-075, forge-wasm's `edit`).

export const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

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

