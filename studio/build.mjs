// Put Forge Studio together in one folder that any static host can serve:
// the page, forge-wasm built for the web, and the example models of this
// repository's examples/ (ADR-056).
//
//   node studio/build.mjs <wasm-bindgen --target web output> <out dir>

import { createHash } from "node:crypto";
import { readFileSync, writeFileSync, mkdirSync, copyFileSync, readdirSync, existsSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const examples = join(here, "..", "examples");

// Each set opens on its main model. `case` names the forge-wasm case
// (forge-wasm/tests/cases.json) that calculates the main model natively; the
// screen test compares the page's numbers with it.
export const SETS = [
  { id: "bagel", title: "Bagel Lune", case: "calculate bagel", dir: "bagel", main: "bagel.yaml",
    blurb: "A bagel shop's first year: seasonal months, costs in an included file, scenarios, a tornado, the year's actuals, and an uncertain version to simulate.",
    files: ["bagel.yaml", "costs.yaml", "actual.yaml", "bagel_simulate.yaml", "costs_risk.yaml", "second_shop.yaml", "wait_option.yaml", "first_month.yaml", "slow_week.yaml"] },
  { id: "mortgage", title: "Canadian mortgage", case: "calculate mortgage", dir: "mortgage", main: "mortgage.yaml",
    blurb: "450,000 at 4.79% over 25 years, and what more a month saves: a 300-month schedule where each month opens with the last one's balance.",
    files: ["mortgage.yaml"] },
  { id: "monthly", title: "Monthly plan", case: "calculate monthly plan", dir: "periods", main: "monthly_plan.yaml",
    blurb: "A shop's year by month: a growing trend, a season factor by calendar month, and cash that carries over with PREV.",
    files: ["monthly_plan.yaml"] },
  { id: "dcf", title: "Five-year DCF", case: "calculate dcf", dir: "periods", main: "dcf.yaml",
    blurb: "Revenue that grows from last year's, discount factors by year, a terminal value and the equity value.",
    files: ["dcf.yaml"] },
  { id: "household", title: "Household budget", case: "calculate household", dir: "household", main: "budget.yaml",
    blurb: "One household's month: the budget, and what actually came in and went out, for budget against actual.",
    files: ["budget.yaml", "actual.yaml"] },
  { id: "options", title: "Rentals compared", case: "calculate options", dir: "options", main: "compare-options.yaml",
    blurb: "Rental units with the same keys, lined up side by side against a take-home pay, as forge report shows them.",
    files: ["compare-options.yaml"] },
  { id: "capex", title: "Machine on a loan", case: "calculate capex", dir: "capex-plan", main: "capex.yaml",
    blurb: "A 120,000 machine on a five-year loan: what the bank is paid in year 2, how the machine is written down, and what its revenue grew by.",
    files: ["capex.yaml"] },
  { id: "forecast", title: "Rolling forecast", case: "calculate rolling forecast", dir: "rolling-forecast", main: "forecast.yaml",
    blurb: "A fiscal year from April with actuals to October: months not closed yet are empty, a budget is looked up by month, and the variance splits into volume, mix and price.",
    files: ["forecast.yaml"] },
  { id: "starter", title: "Starter model", case: "calculate starter", dir: "", main: "starter.yaml",
    blurb: "Forge's first model to start from: a group of scalars, a table with a text column, and totals.",
    files: ["starter.yaml"] },
];

// The patterns of the VS Code extension's formula grammar, in its order. The
// page colours formulas with these (studio/text.js), so there is one copy.
export function formulaRules(injection) {
  return injection.repository.formula.patterns.map((p) => ({
    match: p.match,
    scope: p.name ?? null,
    captures: Object.fromEntries(Object.entries(p.captures ?? {}).map(([n, c]) => [n, c.name])),
  }));
}
// sw.js with the list of the site's files and a name made from their content
export function writeServiceWorker(out) {
  const files = [];
  const walk = (dir, prefix) => {
    for (const e of readdirSync(dir, { withFileTypes: true }).sort((a, b) => a.name.localeCompare(b.name))) {
      if (e.isDirectory()) walk(join(dir, e.name), `${prefix}${e.name}/`);
      else if (e.name !== "sw.js" && e.name !== ".nojekyll") files.push(`${prefix}${e.name}`);
    }
  };
  walk(out, "");
  const hash = createHash("sha256");
  for (const f of files) hash.update(f).update(readFileSync(join(out, f)));
  const list = ["./", ...files];
  writeFileSync(join(out, "sw.js"), readFileSync(join(here, "sw.js"), "utf8")
    .replace("__BUILD__", hash.digest("hex").slice(0, 12))
    .replace("__FILES__", JSON.stringify(list, null, 2)));
}

export const GRAMMAR = join(here, "..", "editors", "vscode", "syntaxes", "forge-formula.injection.json");

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const [pkg, out] = process.argv.slice(2).map((p) => resolve(p));
  if (!pkg || !out) {
    console.error("usage: node studio/build.mjs <pkg dir> <out dir>");
    process.exit(2);
  }
  mkdirSync(join(out, "pkg"), { recursive: true });
  for (const f of readdirSync(here)) {
    if (/\.(html|css|js|svg|png|webmanifest)$/.test(f) && f !== "sw.js") copyFileSync(join(here, f), join(out, f));
  }
  for (const f of ["forge_wasm.js", "forge_wasm_bg.wasm"]) {
    if (!existsSync(join(pkg, f))) { console.error(`${f} is not in ${pkg}: build forge-wasm with wasm-bindgen --target web`); process.exit(1); }
    copyFileSync(join(pkg, f), join(out, "pkg", f));
  }
  const sets = SETS.map(({ dir, files, ...set }) => ({
    ...set,
    files: Object.fromEntries(files.map((f) => [f, readFileSync(join(examples, dir, f), "utf8")])),
  }));
  writeFileSync(join(out, "examples.json"), JSON.stringify(sets));
  writeFileSync(join(out, "formula-rules.json"), JSON.stringify(formulaRules(JSON.parse(readFileSync(GRAMMAR, "utf8")))));
  // GitHub Pages runs Jekyll on a site unless told not to
  writeFileSync(join(out, ".nojekyll"), "");
  writeServiceWorker(out);
  console.log(`Forge Studio in ${out}: ${sets.length} example sets`);
}
