// Text functions on YAML files. Nothing here reads a model's meaning: it
// finds a line and changes it, or colors the text for the editor. Forge reads
// the model (studio/forge.js).

export const esc = (s) => String(s).replace(/[&<>"]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" }[c]));

// Change one input's value in a file's text and nothing else, keeping comments
// and layout. It handles the three ways an input is written: `name: 16`,
// `name: {value: 16, ...}` and a `value: 16` line under `name:`. `group` is
// the scalar's group (null at the top level). Answers null when the line is
// not found, and the caller says so instead of rewriting the file.
export function setInputText(text, group, name, v) {
  const lines = text.split("\n");
  const keyRe = (k) => new RegExp(`^(\\s*)("${k}"|'${k}'|${k.replace(/[.*+?^${}()|[\]\\]/g, "\\$&")}):(.*)$`);
  let start = 0, indent = -1;
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
    indent = m[1].length;
    const rest = m[3];
    const sm = /^(\s*)(-?[\d.eE+]+)(\s*(#.*)?)$/.exec(rest);
    if (sm) { lines[i] = m[1] + m[2] + ":" + sm[1] + v + sm[3]; return lines.join("\n"); }
    const fm = /^(\s*\{.*?\bvalue:\s*)(-?[\d.eE+]+|null|~)(.*)$/.exec(rest);
    if (fm) { lines[i] = m[1] + m[2] + ":" + fm[1] + v + fm[3]; return lines.join("\n"); }
    for (let j = i + 1; j < lines.length; j++) {
      const lj = lines[j];
      if (lj.trim() && !lj.trim().startsWith("#") && lj.match(/^\s*/)[0].length <= indent) break;
      const vm = /^(\s+value:\s*)([^#]*?)(\s*#.*)?$/.exec(lj);
      if (vm) { lines[j] = vm[1] + v + (vm[3] || ""); return lines.join("\n"); }
    }
    return null;
  }
  return null;
}

// Formula rules ported from Forge's VS Code extension (editors/vscode/syntaxes), in its order:
// text, function call, @include value, TRUE/FALSE, name, number, operator, punctuation.
// The YAML around them (keys, comments, strings, numbers) is a small line scanner.
export const FORMULA_RULES = [
  [/^(?:\\"(?:[^"\\]|\\[^"])*\\"|"[^"]*")/, "t-str"],
  [/^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*(?=\s*\()/, "t-fn"],
  [/^@[A-Za-z_][A-Za-z0-9_]*(?:\.@?[A-Za-z_][A-Za-z0-9_]*)+/, "t-inc"],
  [/^(?:TRUE|FALSE)\b/i, "t-const"],
  [/^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*\b/, "t-name"],
  [/^\d*\.?\d+(?:[eE][-+]?\d+)?%?/, "t-num"],
  [/^(?:<=|>=|<>|[-+*/^&=<>])/, "t-op"],
  [/^[(),;:\[\]{}]/, "t-punct"],
];
const span = (cls, s) => (s ? `<span class="${cls}">${esc(s)}</span>` : "");
function hlFormula(src, quote) {
  let out = span("t-eq", "="), i = 1;
  while (i < src.length) {
    const rest = src.slice(i);
    if (/^\s/.test(rest)) { const m = /^\s+/.exec(rest)[0]; out += esc(m); i += m.length; continue; }
    if (quote === "'" && rest.startsWith('"')) { const j = src.indexOf('"', i + 1); const s = j < 0 ? rest : src.slice(i, j + 1); out += span("t-str", s); i += s.length; continue; }
    let hit = null;
    for (const [re, cls] of FORMULA_RULES) { const m = re.exec(rest); if (m && m[0]) { hit = [m[0], cls]; break; } }
    if (!hit) { out += esc(rest[0]); i++; continue; }
    out += span(hit[1], hit[0]); i += hit[0].length;
  }
  return out;
}
function hlQuoted(s, q) {
  const body = s.slice(1, s.endsWith(q) && s.length > 1 ? -1 : undefined);
  const close = s.length > 1 && s.endsWith(q) ? q : "";
  if (body.startsWith("=")) return span("t-q", q) + `<span class="t-formula">${hlFormula(body, q)}</span>` + span("t-q", close);
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
      out += `<span class="t-formula">${hlFormula(f, "")}</span>`; i += f.length; continue;
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

