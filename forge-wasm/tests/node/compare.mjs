// Run every case of tests/cases.json through the WebAssembly build and compare
// each answer with the native build's (examples/expected.rs). Numbers may
// differ in their last bits (the browser's maths library is not the system's);
// anything else must be equal.
//
//   node compare.mjs <pkg dir, wasm-bindgen --target nodejs> <expected.json> <examples dir>

import { createRequire } from "node:module";
import { readFileSync, statSync, appendFileSync } from "node:fs";
import { gzipSync } from "node:zlib";
import { join, resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { differences as workbookDifferences } from "./xlsx.mjs";

const [pkg, expectedPath, examples] = process.argv.slice(2).map((p) => resolve(p));
const here = dirname(fileURLToPath(import.meta.url));
const forge = createRequire(import.meta.url)(join(pkg, "forge_wasm.js"));
const cases = JSON.parse(readFileSync(join(here, "..", "cases.json"), "utf8"));
const expected = JSON.parse(readFileSync(expectedPath, "utf8"));

const RELATIVE = 1e-9;
const differences = [];

function compare(a, b, path) {
  if (typeof a === "number" && typeof b === "number") {
    if (Math.abs(a - b) > RELATIVE * Math.max(1, Math.abs(a), Math.abs(b))) {
      differences.push(`${path}: wasm ${a}, native ${b}`);
    }
    return;
  }
  if (Array.isArray(a) && Array.isArray(b)) {
    if (a.length !== b.length) return differences.push(`${path}: ${a.length} items, native ${b.length}`);
    a.forEach((x, i) => compare(x, b[i], `${path}[${i}]`));
    return;
  }
  if (a && b && typeof a === "object" && typeof b === "object") {
    const keys = new Set([...Object.keys(a), ...Object.keys(b)]);
    keys.delete("execution_time_ms");
    // A workbook is compared by its parts (xlsx.mjs): it records when it was made
    if ("excel_base64" in a && "excel_base64" in b) {
      keys.delete("excel_base64");
      keys.delete("byte_count");
      for (const d of workbookDifferences(Buffer.from(a.excel_base64, "base64"), Buffer.from(b.excel_base64, "base64"))) differences.push(`${path}: ${d}`);
    }
    for (const k of keys) {
      if (!(k in a) || !(k in b)) differences.push(`${path}.${k}: only in ${k in a ? "wasm" : "native"}`);
      else compare(a[k], b[k], `${path}.${k}`);
    }
    return;
  }
  if (a !== b) differences.push(`${path}: wasm ${JSON.stringify(a)}, native ${JSON.stringify(b)}`);
}

const rows = [];
for (const c of cases) {
  const request = { ...(c.args ?? {}) };
  if (c.files) {
    request.files = Object.fromEntries(
      c.files.map((name) => [name, readFileSync(join(examples, c.dir, name), "utf8")]),
    );
  }
  if (c.main) request.main = c.main;
  console.log(`running: ${c.name}`);
  const start = performance.now();
  const answer = JSON.parse(forge.call(c.tool, JSON.stringify(request)));
  const ms = performance.now() - start;
  const before = differences.length;
  compare(answer, expected[c.name], c.name);
  rows.push({ name: c.name, ms, ok: differences.length === before && !("error" in answer) });
}

const wasm = join(pkg, "forge_wasm_bg.wasm");
const bytes = statSync(wasm).size;
const gzipped = gzipSync(readFileSync(wasm), { level: 9 }).length;
const mb = (n) => (n / 1048576).toFixed(2);
const summary = [
  `## forge-wasm ${forge.version()}`,
  "",
  `${rows.filter((r) => r.ok).length} of ${rows.length} cases give the native answer.`,
  "",
  `WebAssembly module: ${mb(bytes)} MB, ${mb(gzipped)} MB gzipped (wasm-bindgen output, not run through wasm-opt).`,
  "",
  "| Case | Time in Node (ms) | Same as native |",
  "| --- | ---: | --- |",
  ...rows.map((r) => `| ${r.name} | ${r.ms.toFixed(0)} | ${r.ok ? "yes" : "**no**"} |`),
].join("\n");

console.log(summary);
if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, summary + "\n");
if (differences.length) {
  console.error(`\n${differences.length} differences:\n` + differences.slice(0, 50).join("\n"));
  process.exit(1);
}
