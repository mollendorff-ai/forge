// Put Forge Studio together in one folder that any static host can serve:
// the page, forge-wasm built for the web, and the example models of this
// repository's examples/ (ADR-056).
//
//   node studio/build.mjs <wasm-bindgen --target web output> <out dir>

import { readFileSync, writeFileSync, mkdirSync, copyFileSync, readdirSync, existsSync } from "node:fs";
import { join, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const examples = join(here, "..", "examples");
const [pkg, out] = process.argv.slice(2).map((p) => resolve(p));
if (!pkg || !out) {
  console.error("usage: node studio/build.mjs <pkg dir> <out dir>");
  process.exit(2);
}

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
  { id: "starter", title: "Starter model", case: "calculate starter", dir: "", main: "starter.yaml",
    blurb: "Forge's first model to start from: a group of scalars, a table with a text column, and totals.",
    files: ["starter.yaml"] },
];

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  mkdirSync(join(out, "pkg"), { recursive: true });
  for (const f of readdirSync(here)) {
    if (/\.(html|css|js)$/.test(f)) copyFileSync(join(here, f), join(out, f));
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
  // GitHub Pages runs Jekyll on a site unless told not to
  writeFileSync(join(out, ".nojekyll"), "");
  console.log(`Forge Studio in ${out}: ${sets.length} example sets`);
}
