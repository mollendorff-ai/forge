// Forge Studio holds no calculation code of its own (ADR-055 decision 6,
// roadmap S3): every number comes from forge-wasm. This test fails when a
// file of the page loads a YAML library, carries the mockup's JavaScript
// engine, or implements what Forge calculates.
//
//   node studio/tests/no-engine.mjs

import { readFileSync, readdirSync } from "node:fs";
import { join, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const studio = join(dirname(fileURLToPath(import.meta.url)), "..");
const FORBIDDEN = [
  [/js-?yaml|yaml\.(load|parse)\s*\(/i, "a YAML library: Forge reads the model"],
  [/ForgeEngine|engine\.js/, "the mockup's JavaScript engine"],
  [/\bfunction\s+(npv|irr|pmt|fv|pv|sumproduct|vlookup|evaluate|tokenize|parseFormula)\b/i, "a spreadsheet function or formula evaluator"],
  [/Math\.(pow|exp|log|random)\b|\*\*/, "maths that belongs to Forge (powers, logarithms, random draws)"],
  [/\bnew Function\b|\beval\s*\(/, "code built from text"],
];

const files = readdirSync(studio).filter((f) => /\.(js|mjs|html)$/.test(f) && f !== "build.mjs");
const found = [];
for (const f of files) {
  readFileSync(join(studio, f), "utf8").split("\n").forEach((line, i) => {
    for (const [re, what] of FORBIDDEN) if (re.test(line)) found.push(`studio/${f}:${i + 1}: ${what}\n    ${line.trim()}`);
  });
}
if (found.length) {
  console.error(`Forge Studio must not calculate (${found.length}):\n` + found.join("\n"));
  process.exit(1);
}
console.log(`No calculation code in ${files.length} files of studio/: ${files.join(", ")}`);
