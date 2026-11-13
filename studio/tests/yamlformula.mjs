// Names and functions inside a formula typed in the YAML editor, in a headless
// browser with Forge's own answers:
//
// - in every formula of every file of the examples that is part of a model (the
//   main file and the files it includes), typing the first three letters of
//   each name or function in it, at its place in the file, offers that name or
//   function as the formula writes it, and taking it gives the same word;
// - nothing is offered after the formula is closed, in a comment, or for a value
//   that is not a formula, and keys are not offered inside a formula;
// - in the editor, the strip offers a function and a name, a tap inserts exactly
//   its text, and the editor then holds the file as typed.
//
//   node studio/tests/yamlformula.mjs <built Studio dir or address>

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
  const { functionsOf, suggestInFormula, formulaContext, suggestKeys, schemaOf, apply } = await import("./suggest.js");
  const sets = await (await fetch("examples.json")).json();
  const functions = functionsOf(await call("functions", {}));
  const schema = schemaOf(await call("schema", { version: "v5.1.0" }));
  const problems = [];
  let formulas = 0, words = 0, offered = 0, files = 0;
  // A formula in a file: `key: "=..."` or `key: '=...'`
  const LINE = /^(\s*)(-\s+)?(?:"[^"]+"|'[^']+'|[^\s:#"'-][^:#"']*?)\s*:\s+(["'])(=.*)\3\s*(?:#.*)?$/;
  const wordsOf = (formula) => {
    const plain = formula.replace(/\\"|""/g, "  ").replace(/"(?:[^"\\]|\\.)*"/g, (m) => '"' + " ".repeat(m.length - 2) + '"');
    return [...plain.matchAll(/(?<![A-Za-z0-9_.@])(@?[A-Za-z_][A-Za-z0-9_.@]*)(\()?/g)].map((m) => ({ word: m[1], call: !!m[2], index: m.index }));
  };
  for (const set of sets) {
    const model = await call("model", { files: set.files, main: set.main });
    for (const [name, text] of Object.entries(set.files)) {
      const isMain = name === set.main, included = model.includes.some((i) => i.file === name);
      if (!isMain && !included) continue;
      files++;
      const lines = text.split("\n");
      for (let i = 0; i < lines.length; i++) {
        const m = LINE.exec(lines[i]);
        if (!m) continue;
        formulas++;
        const formula = m[4];
        const start = lines[i].indexOf(m[3] + "=", m[0].indexOf(":")) + 1;
        const above = lines.slice(0, i).join("\n") + (i ? "\n" : "");
        for (const { word, call: isCall, index } of wordsOf(formula)) {
          words++;
          const typed = word.slice(0, Math.min(3, word.length));
          const upto = above + lines[i].slice(0, start + index) + typed;
          const list = suggestInFormula({ structure: model, functions, file: name }, upto, upto.length);
          const want = isCall ? word + "(" : word;
          const hit = list && list.find((s) => s.label === want || (!isCall && s.insert === word));
          if (!hit) { problems.push(`${set.id}/${name}:${i + 1}: typing "${typed}" does not offer ${want}${list ? ` (offers ${list.map((s) => s.label).join(" ") || "nothing"})` : " (not seen as a formula)"}`); continue; }
          offered++;
          const after = apply(upto, hit).text;
          if (after !== above + lines[i].slice(0, start + index) + want) problems.push(`${set.id}/${name}:${i + 1}: taking ${hit.label} after "${typed}" gives a different file`);
        }
        // Closed, or after the formula: nothing; a comment: nothing
        const closed = above + lines[i];
        if (suggestInFormula({ structure: model, functions, file: name }, closed, closed.length)) problems.push(`${set.id}/${name}:${i + 1}: seen as a formula after it is closed`);
      }
      // A value that is not a formula, a comment, and keys inside a formula
      for (let i = 0; i < lines.length; i++) {
        const l = lines[i];
        const h = l.indexOf("# ");
        const cut = (n) => lines.slice(0, i).concat(l.slice(0, n)).join("\n");
        if (h >= 0 && /^[A-Za-z_]/.test(l.slice(h + 2, h + 4))) { const t = cut(h + 4); if (suggestInFormula({ structure: model, functions, file: name }, t, t.length)) problems.push(`${set.id}/${name}:${i + 1}: offered in a comment`); }
        const c = l.indexOf(": ");
        if (c > 0 && /^[A-Za-z_0-9]/.test(l.slice(c + 2, c + 4)) && !/^["'=]/.test(l.slice(c + 2))) { const t = cut(c + 4); if (suggestInFormula({ structure: model, functions, file: name }, t, t.length)) problems.push(`${set.id}/${name}:${i + 1}: offered for a value that is not a formula`); }
      }
    }
  }
  // Keys are not offered inside a formula
  const inFormula = 'year:\n  margin:\n    formula: "=un';
  if (suggestKeys(schema, inFormula, inFormula.length).length) problems.push("keys are offered inside a formula");
  if (!formulaContext(inFormula, inFormula.length)) problems.push("a formula is not seen as one");
  const bare = 'year:\n  margin:\n    formula: =un';
  if (suggestKeys(schema, bare, bare.length).length) problems.push("keys are offered inside an unquoted formula");
  if (formulaContext(bare, bare.length)?.formula !== "=un") problems.push("an unquoted formula is not seen as one");
  return { problems, formulas, words, offered, files };
});
for (const p of result.problems.slice(0, 40)) fail(p);
if (result.problems.length > 40) fail(`… and ${result.problems.length - 40} more`);
if (result.words < 200) fail(`only ${result.words} words were tested`);

// In the editor
await page.click('.tab[data-tab="yaml"]');
const editor = page.locator("#yaml-editor");
const strip = page.locator("#view-yaml [data-suggest]");
const original = await editor.inputValue();
const type = async (text) => {
  await editor.fill(text);
  await editor.evaluate((el) => { el.setSelectionRange(el.value.length, el.value.length); });
  await editor.dispatchEvent("input");
};
const head = original.replace(/\s*$/, "\n");
await type(head + 'year:\n  margin2:\n    formula: "=SU');
await strip.locator("[data-sug]", { hasText: /^SUM\($/ }).waitFor({ timeout: 30000 }).catch(() => {});
const fns = await strip.locator("[data-sug]").allTextContents();
if (!fns.includes("SUM(")) fail(`typing "SU" in a formula in the editor offers ${fns.join(" ") || "nothing"}, not SUM(`);
await strip.locator("[data-sug]", { hasText: /^SUM\($/ }).click();
if ((await editor.inputValue()) !== head + 'year:\n  margin2:\n    formula: "=SUM(') fail(`taking SUM( gave ${JSON.stringify((await editor.inputValue()).slice(-40))}`);
await type(head + 'year:\n  margin2:\n    formula: "=profit / re');
await strip.locator("[data-sug]", { hasText: /^revenue$/ }).waitFor({ timeout: 30000 }).catch(() => {});
if (!(await strip.locator("[data-sug]").allTextContents()).includes("revenue")) fail('typing "re" after profit / in the editor does not offer revenue');
await strip.locator("[data-sug]", { hasText: /^revenue$/ }).click();
if (!(await editor.inputValue()).endsWith('formula: "=profit / revenue')) fail("taking revenue did not give the name as typed");
if (!(await page.locator("#yaml-apply").isEnabled())) fail("after taking a name the editor is not marked as edited");
await type(head + 'year:\n  margin2:\n    formula: "=profit / revenue"\n    un');
await strip.locator("[data-sug]", { hasText: /^unit:$/ }).waitFor({ timeout: 30000 }).catch(() => {});
if (!(await strip.locator("[data-sug]").allTextContents()).includes("unit:")) fail("after a closed formula the keys are not offered again");

// A formula written without quotes, as the strip leaves it after `formula: `
await type(head + 'year:\n  margin2:\n    formula: =profit / re');
await strip.locator("[data-sug]", { hasText: /^revenue$/ }).waitFor({ timeout: 30000 }).catch(() => {});
if (!(await strip.locator("[data-sug]").allTextContents()).includes("revenue")) fail("an unquoted formula is not offered names");
await strip.locator("[data-sug]", { hasText: /^revenue$/ }).click();
if (!(await editor.inputValue()).endsWith("formula: =profit / revenue")) fail("taking revenue in an unquoted formula did not give the name as typed");
await type(head + 'year:\n  margin2:\n    formula: =profit # re');
if ((await strip.locator("[data-sug]").allTextContents()).includes("revenue")) fail("names are offered in a comment after an unquoted formula");

await browser.close();
if (!remote) server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`YAML formulas: ${result.offered} of ${result.words} names and functions in ${result.formulas} formulas of ${result.files} example files are offered by their first three letters at their place in the file and complete to the same word; nothing after the formula, in a comment or for a plain value; keys are not offered inside a formula; the strip in the editor inserts exactly the text.`);
