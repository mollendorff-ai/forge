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
    blurb: "A bagel shop's first year: seasonal months, costs in an included file, scenarios, a tornado, the year's actuals, an uncertain version to simulate, a price war with the shop across the street that spreads through the city, and the street's five shops deciding whether to fund a festival.",
    files: ["bagel.yaml", "costs.yaml", "actual.yaml", "bagel_simulate.yaml", "costs_risk.yaml", "second_shop.yaml", "wait_option.yaml", "first_month.yaml", "slow_week.yaml", "price_war.yaml", "street_fund.yaml"] },
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
  { id: "microgrid", title: "Island microgrid", case: "calculate microgrid", dir: "microgrid", main: "microgrid.yaml",
    blurb: "An island trading diesel for solar and a battery over 20 years: how much solar the battery lets in, its replacement in year 11, the loan and the scenarios.",
    files: ["microgrid.yaml", "microgrid_risk.yaml", "microgrid_decision.yaml"] },
  { id: "realestate", title: "Linden Court", case: "calculate realestate", dir: "realestate", main: "realestate.yaml",
    blurb: "A building of 24 flats valued under uncertainty: ten years of rent and costs, a sale at an exit cap rate, a loan, scenarios, a tornado, a version to simulate and the flexibility to sell early.",
    files: ["realestate.yaml", "realestate_risk.yaml", "realestate_flex.yaml"] },
  { id: "acquisition", title: "Project Kittiwake", case: "calculate acquisition", dir: "acquisition", main: "deal.yaml",
    blurb: "A listed company bids 30% over the market for a smaller rival, part cash, part shares: value created against the premium, earnings per share year by year, the walk-away price, an earn-out, a rival bidder, diligence and the deal a year after closing.",
    files: ["deal.yaml", "buyer.yaml", "target.yaml", "deal_risk.yaml", "deal_bid.yaml", "deal_game.yaml", "halo_option.yaml", "premiums.yaml", "diligence.yaml", "actual.yaml"] },
  { id: "fund", title: "Corvane Seed Fund I", case: "calculate fund", dir: "fund", main: "fund.yaml",
    blurb: "A venture fund seen by its investors: calls, fees and exits on their own dates, the return by XIRR and the value by XNPV, and what a year's delay or a cold exit market costs.",
    files: ["fund.yaml"] },
  { id: "vineyard", title: "Les Trois Pentes", case: "calculate vineyard", dir: "vineyard", main: "vineyard.yaml",
    blurb: "A vineyard's season, where a big harvest sells cheap: the crop and the price correlated, a spring frost one year in ten, scenarios, a tornado, and the same season simulated with and without the correlation.",
    files: ["vineyard.yaml", "costs.yaml", "vineyard_risk.yaml", "vineyard_independent.yaml"] },
  { id: "games", title: "Games", case: "calculate stag hunt", dir: "games", main: "stag-hunt.yaml",
    blurb: "Games played once and again and again: a stag hunt, chicken, the battle of the sexes and an inspection, each with its equilibria marked; a public goods game and a volunteer's dilemma of five players; hawks and doves spreading through a population; and Axelrod's tournament, with mistakes and with a chance of another round.",
    files: ["stag-hunt.yaml", "chicken.yaml", "axelrod.yaml", "inspection.yaml", "noisy-tournament.yaml", "battle-of-the-sexes.yaml", "continuation.yaml", "public-goods.yaml", "volunteers-dilemma.yaml", "hawk-dove.yaml"] },
  { id: "lender", title: "A lender's loan", case: "calculate expected loss", dir: "", main: "expected_loss.yaml",
    blurb: "A loan that defaults more often when revenue is low, and revenue is low more often when the economy is bad: a Bayesian network whose chances feed the expected loss, with a recession and a boom to read it in.",
    files: ["expected_loss.yaml"] },
  { id: "starter", title: "Starter model", case: "calculate starter", dir: "", main: "starter.yaml",
    blurb: "Forge's first model to start from: a group of scalars, a table with a text column, and totals.",
    files: ["starter.yaml"] },
];

// A set of several models shows them as questions, in this order, on the
// welcome card and in the strip under the title (ADR-077): each is [file,
// short name for the strip, question]. A file only included by another
// (costs.yaml) is not a model of its own and is not listed. The badge beside
// each comes from Forge (`model` answers `analyses`), not from here.
export const QUESTIONS = {
  bagel: [
    ["bagel.yaml", "plan", "Will the first year make money?"],
    ["bagel_simulate.yaml", "how sure", "How sure can we be of the year?"],
    ["price_war.yaml", "price war", "Cut the price if the rival does?"],
    ["street_fund.yaml", "festival", "Will the street pay for a festival?"],
    ["second_shop.yaml", "second shop", "Open a second shop, or try a pop-up first?"],
    ["wait_option.yaml", "wait a year", "What is waiting a year before the second lease worth?"],
    ["first_month.yaml", "first month", "How sure are we of the average day?"],
    ["slow_week.yaml", "slow week", "A slow week: the rain, or the roadwork?"],
    ["actual.yaml", "actual", "What actually happened in the first year?"],
  ],
  household: [
    ["budget.yaml", "budget", "Where does October's money go?"],
    ["actual.yaml", "actual", "What actually came in and went out?"],
  ],
  microgrid: [
    ["microgrid.yaml", "plan", "Should the island trade its diesel for solar and a battery?"],
    ["microgrid_risk.yaml", "how sure", "How sure is the saving?"],
    ["microgrid_decision.yaml", "which system", "Which system, and build now or wait a year?"],
  ],
  realestate: [
    ["realestate.yaml", "plan", "What is the building worth, held ten years and sold?"],
    ["realestate_risk.yaml", "how sure", "And with rents, vacancy and costs uncertain?"],
    ["realestate_flex.yaml", "sell early", "What is the freedom to sell early worth?"],
  ],
  acquisition: [
    ["deal.yaml", "deal", "Does the deal create value for the buyer?"],
    ["deal_risk.yaml", "how sure", "And if synergies and integration are uncertain?"],
    ["deal_bid.yaml", "bid", "Open at 30% and risk a counter-bid, or go in at 40%?"],
    ["deal_game.yaml", "sealed bids", "Sealed bids against a rival fund: 30% or 40%?"],
    ["halo_option.yaml", "Halo", "What is the platform in the lab worth?"],
    ["premiums.yaml", "premiums", "Is a 30% premium enough to win the board?"],
    ["diligence.yaml", "diligence", "A spike in sales: a big contract, or channel stuffing?"],
    ["actual.yaml", "a year on", "How does the deal look a year after closing?"],
  ],
  vineyard: [
    ["vineyard.yaml", "plan", "What does one season earn?"],
    ["vineyard_risk.yaml", "how sure", "How sure is the season, frost and all?"],
    ["vineyard_independent.yaml", "independent", "And if the price did not follow the harvest?"],
  ],
  games: [
    ["stag-hunt.yaml", "stag hunt", "Hunt the stag together, or a hare alone?"],
    ["battle-of-the-sexes.yaml", "battle of the sexes", "The opera or the football?"],
    ["chicken.yaml", "chicken", "Who swerves?"],
    ["inspection.yaml", "inspection", "How often to audit, if cheating pays?"],
    ["hawk-dove.yaml", "hawk–dove", "Fight or share, and which spreads?"],
    ["public-goods.yaml", "public goods", "Five players: put in, or keep?"],
    ["volunteers-dilemma.yaml", "volunteers", "Five players: who runs the nightly check?"],
    ["axelrod.yaml", "Axelrod", "Which habit wins a dilemma played again and again?"],
    ["noisy-tournament.yaml", "noise", "And when moves are sometimes mistaken?"],
    ["continuation.yaml", "continuation", "How likely must a next round be for cooperation to hold?"],
  ],
};

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
    models: QUESTIONS[set.id] ?? null,
    files: Object.fromEntries(files.map((f) => [f, readFileSync(join(examples, dir, f), "utf8")])),
  }));
  writeFileSync(join(out, "examples.json"), JSON.stringify(sets));
  // The help sheets' text (ADR-077)
  copyFileSync(join(here, "help.json"), join(out, "help.json"));
  writeFileSync(join(out, "formula-rules.json"), JSON.stringify(formulaRules(JSON.parse(readFileSync(GRAMMAR, "utf8")))));
  // GitHub Pages runs Jekyll on a site unless told not to
  writeFileSync(join(out, ".nojekyll"), "");
  writeServiceWorker(out);
  console.log(`Forge Studio in ${out}: ${sets.length} example sets`);
}
