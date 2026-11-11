// Forge Studio colours formulas with the VS Code extension's rules (roadmap
// S6, ADR-056 decision 4). Fails when:
//
// - the rules the page loads (formula-rules.json, written by build.mjs) differ
//   from the patterns of editors/vscode/syntaxes/forge-formula.injection.json
//   in the patterns, their order or their scopes;
// - the extension gets a scope Studio has no colour for, or a pattern
//   JavaScript cannot run;
// - on any YAML file of examples/, the highlighted text with its tags
//   removed is not the file;
// - a few formulas do not get the classes the grammar gives them.
//
//   node studio/tests/highlight.mjs [built Studio dir]

import { readFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { SCOPE_CLASS, compileRule, useFormulaRules, highlightYaml } from "../text.js";
import { formulaRules, GRAMMAR } from "../build.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const failures = [];
const fail = (m) => failures.push(m);

// 1. The rules are the grammar's
const grammar = JSON.parse(readFileSync(GRAMMAR, "utf8"));
const patterns = grammar.repository.formula.patterns;
const rules = formulaRules(grammar);
if (rules.length !== patterns.length) fail(`${rules.length} rules from ${patterns.length} patterns`);
patterns.forEach((p, i) => {
  if (!rules[i] || rules[i].match !== p.match) fail(`rule ${i + 1} is not the grammar's pattern ${p.match}`);
});
const built = process.argv[2] && resolve(process.argv[2], "formula-rules.json");
if (built) {
  if (!existsSync(built)) fail(`${built} does not exist: build Studio first`);
  else if (JSON.stringify(JSON.parse(readFileSync(built, "utf8"))) !== JSON.stringify(rules)) fail("formula-rules.json in the built site is not the grammar's patterns, in the grammar's order");
}

// 2. Every scope has a colour; every pattern runs in JavaScript
for (const p of patterns) {
  const scopes = [p.name, ...Object.values(p.captures ?? {}).map((c) => c.name)].filter(Boolean);
  if (!scopes.length) fail(`the pattern ${p.match} has no scope`);
  for (const scope of scopes) if (!(scope in SCOPE_CLASS)) fail(`the grammar's scope ${scope} has no colour in studio/text.js (SCOPE_CLASS)`);
  try { compileRule({ match: p.match }); } catch (e) { fail(`JavaScript cannot run the pattern ${p.match}: ${e.message}`); }
}
const used = new Set(patterns.flatMap((p) => [p.name, ...Object.values(p.captures ?? {}).map((c) => c.name)]));
for (const scope of Object.keys(SCOPE_CLASS)) if (!used.has(scope) && scope !== "keyword.operator.formula.forge") fail(`SCOPE_CLASS has ${scope}, which the grammar no longer has`);

useFormulaRules(rules);

// 3. Taking the tags out gives the file back
const unescape = (t) => t.replace(/<[^>]*>/g, "").replace(/&quot;/g, '"').replace(/&lt;/g, "<").replace(/&gt;/g, ">").replace(/&amp;/g, "&");
const examples = join(here, "..", "..", "examples");
const files = [];
(function walk(dir) {
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) walk(path);
    else if (/\.ya?ml$/.test(name)) files.push(path);
  }
})(examples);
let formulas = 0;
for (const file of files) {
  const text = readFileSync(file, "utf8");
  const html = highlightYaml(text);
  formulas += (html.match(/class="t-formula"/g) || []).length;
  if (unescape(html) !== text + "\n") {
    const a = unescape(html).split("\n"), b = (text + "\n").split("\n");
    const line = a.findIndex((l, i) => l !== b[i]);
    fail(`${file.slice(examples.length - 8)}: line ${line + 1} changed by highlighting:\n    file: ${b[line]}\n    got:  ${a[line]}`);
  }
}
if (formulas < 50) fail(`only ${formulas} formulas were highlighted in ${files.length} files; the test is not looking at the examples`);

// 4. Formulas get the grammar's classes
const classes = (formula) => [...highlightYaml(`x: "${formula}"`).matchAll(/<span class="(t-[a-z]+)">([^<]*)<\/span>/g)].map((m) => `${m[1]}:${unescape(m[2])}`).join(" ");
const expect = (formula, expected) => { const got = classes(formula); if (!got.includes(expected)) fail(`"${formula}" gives ${got}, which lacks ${expected}`); };
expect("=SUM(months.profit)", "t-fn:SUM");
expect("=SUM(months.profit)", "t-name:months.profit");
expect("=a + 1.5%", "t-num:1.5%");
expect("=@costs.labour.wage * 2", "t-inc:@costs.labour.wage");
expect("=IF(x, true, FALSE)", "t-const:true");
expect("=IF(x, true, FALSE)", "t-const:FALSE");
expect("=trueish", "t-name:trueish");           // \b: only the whole word is a boolean
expect('=IF(a, \\"yes\\", \\"no\\")', 't-str:\\"yes\\"');
expect("=MC.Normal(1, 2)", "t-fn:MC.Normal");
expect("=a1.5", "t-name:a1");                    // a number does not start inside a name

if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Formula colours are the VS Code grammar's: ${patterns.length} patterns in its order, ${files.length} example files unchanged by highlighting (${formulas} formulas).`);
