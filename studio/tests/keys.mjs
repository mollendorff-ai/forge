// Suggesting the keys of the model format in the YAML editor (roadmap S12), in
// a headless browser with Forge's own schema:
//
// - in every YAML file of every example, for each line whose key is one the
//   schema names (value, unit, formula, file, probability…), typing its first
//   two letters at that place in the file offers it;
// - only keys the schema names are ever offered;
// - nothing is offered after a colon, inside a formula or a text, or in a
//   comment;
// - in the editor, the strip above it offers the key, a tap inserts exactly
//   its text, and the editor then holds the file as typed.
//
//   node studio/tests/keys.mjs <built Studio dir or address>

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";

const [where] = process.argv.slice(2);
const remote = /^https?:/.test(where);
const dir = resolve(where);
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".wasm": "application/wasm" };
const server = createServer((req, res) => {
  let file = join(dir, decodeURIComponent(new URL(req.url, "http://x").pathname));
  if (existsSync(file) && statSync(file).isDirectory()) file = join(file, "index.html");
  if (!file.startsWith(dir) || !existsSync(file)) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { "content-type": TYPES[extname(file)] || "application/octet-stream" });
  res.end(readFileSync(file));
});
if (!remote) await new Promise((r) => server.listen(0, "127.0.0.1", r));
const base = remote ? where.replace(/\/$/, "") : `http://127.0.0.1:${server.address().port}`;

const browser = await chromium.launch();
const failures = [];
const fail = (m) => failures.push(m);
const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
page.on("pageerror", (e) => fail(e.message));
await page.goto(`${base}/`);
await page.waitForFunction(() => document.body.dataset.state === "ready", null, { timeout: 120000 });

const result = await page.evaluate(async () => {
  const { call } = await import("./forge.js");
  const { schemaOf, suggestKeys, allKeys } = await import("./suggest.js");
  const sets = await (await fetch("examples.json")).json();
  // The keys the format documents (ADR-051, ADR-053, the README): the model's
  // sections, a scalar's metadata, an include, a scenario, a tornado input.
  // The sections the schema leaves to the commands that read them (decision
  // trees, bootstrap, Monte Carlo settings: "checked by forge bootstrap, not by
  // this schema") have no keys to offer, and `name` there is not tested.
  const documented = new Set(["_forge_version", "_name", "_includes", "scenarios", "inputs", "outputs", "monte_carlo", "tornado", "decision_tree", "real_options",
    "value", "unit", "formula", "better", "notes", "file", "as", "probability", "description", "scalars", "low", "high"]);
  const problems = [];
  let tested = 0, files = 0, versions = new Set();
  const schemas = new Map();
  const schemaFor = async (text) => {
    const m = /^_forge_version:\s*["']?(\d+\.\d+\.\d+)/m.exec(text);
    const v = m ? `v${m[1]}` : null;
    if (!v) return null;
    versions.add(v);
    if (!schemas.has(v)) schemas.set(v, schemaOf(await call("schema", { version: v })));
    return schemas.get(v);
  };
  const seen = new Set();
  for (const set of sets) {
    for (const [name, text] of Object.entries(set.files)) {
      if (seen.has(set.id + name) || seen.has(text)) continue;
      seen.add(set.id + name); seen.add(text);
      const schema = await schemaFor(text);
      if (!schema) { problems.push(`${set.id}/${name}: no _forge_version`); continue; }
      files++;
      const keys = allKeys(schema);
      const lines = text.split("\n");
      for (let i = 0; i < lines.length; i++) {
        const m = /^(\s*)(-\s+)?([A-Za-z_][A-Za-z0-9_]*):(\s|$)/.exec(lines[i]);
        if (!m || !keys.has(m[3])) continue;
        // A word that is a name at this place (a scenario called base, a column
        // called start) is not a key; the keys the format documents are
        if (!documented.has(m[3])) continue;
        const typed = m[3].slice(0, Math.min(2, m[3].length));
        const before = lines.slice(0, i).join("\n") + (i ? "\n" : "") + m[1] + (m[2] || "") + typed;
        const list = suggestKeys(schema, before, before.length);
        tested++;
        if (!list.some((s) => s.label === `${m[3]}:`)) { problems.push(`${set.id}/${name}:${i + 1}: typing "${typed}" does not offer ${m[3]}: (offers ${list.map((s) => s.label).join(" ") || "nothing"})`); continue; }
        for (const s of list) if (!keys.has(s.label.slice(0, -1))) problems.push(`${set.id}/${name}:${i + 1}: ${s.label} is not a key of the schema`);
      }
      // Nothing after a colon, inside a formula or a text, or in a comment
      for (let i = 0; i < lines.length; i++) {
        const l = lines[i], c = l.indexOf(": ");
        if (c < 0) continue;
        const rest = l.slice(c + 2);
        const cut = (n) => lines.slice(0, i).concat(l.slice(0, n)).join("\n");
        const afterColon = cut(c + 2 + Math.min(2, rest.length));
        if (/^[A-Za-z_]/.test(rest) && suggestKeys(schema, afterColon, afterColon.length).length) problems.push(`${set.id}/${name}:${i + 1}: offered after a colon`);
        const q = l.indexOf('"=');
        if (q > 0) { const t = cut(q + 4); if (suggestKeys(schema, t, t.length).length) problems.push(`${set.id}/${name}:${i + 1}: offered inside a formula`); }
        const h = l.indexOf("# ");
        if (h >= 0 && /^[A-Za-z_]/.test(l.slice(h + 2, h + 4))) { const t = cut(h + 4); if (suggestKeys(schema, t, t.length).length) problems.push(`${set.id}/${name}:${i + 1}: offered in a comment`); }
      }
    }
  }
  return { problems, tested, files, versions: [...versions] };
});
for (const p of result.problems.slice(0, 40)) fail(p);
if (result.problems.length > 40) fail(`… and ${result.problems.length - 40} more`);
if (result.tested < 100) fail(`only ${result.tested} keys were tested`);

// In the editor
await page.click('.tab[data-tab="yaml"]');
const editor = page.locator("#yaml-editor");
const strip = page.locator("#view-yaml [data-suggest]");
const type = async (text) => {
  await editor.fill(text);
  await editor.evaluate((el) => { el.setSelectionRange(el.value.length, el.value.length); });
  await editor.dispatchEvent("input");
};
await type('_forge_version: "5.1.0"\nassumptions:\n  hours:\n    un');
await strip.locator("[data-sug]", { hasText: /^unit:$/ }).waitFor({ timeout: 30000 }).catch(() => {});
const labels = await strip.locator("[data-sug]").allTextContents();
if (!labels.includes("unit:")) fail(`typing "un" under a scalar offers ${labels.join(" ") || "nothing"}, not unit:`);
await strip.locator("[data-sug]", { hasText: /^unit:$/ }).click();
const value = await editor.inputValue();
if (value !== '_forge_version: "5.1.0"\nassumptions:\n  hours:\n    unit: ') fail(`taking unit: gave ${JSON.stringify(value)}`);
if (!(await page.locator("#yaml-apply").isEnabled())) fail("after taking a key the editor is not marked as edited");
await type('_forge_version: "5.1.0"\n_includes:\n  - fi');
await strip.locator("[data-sug]", { hasText: /^file:$/ }).waitFor({ timeout: 30000 }).catch(() => {});
if (!(await strip.locator("[data-sug]").allTextContents()).includes("file:")) fail('typing "fi" in an include does not offer file:');
await type('_forge_version: "5.1.0"\nyear:\n  margin:\n    formula: "=SU');
if (await strip.locator("[data-sug]").count()) fail("keys are offered inside a formula in the editor");

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Keys: ${result.tested} keys in ${result.files} example files (formats ${result.versions.join(", ")}) are offered by their first two letters, only the schema's keys are offered, never after a colon, in a formula or in a comment; the strip in the editor inserts exactly the key.`);
