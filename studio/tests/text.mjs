// Studio's text changes to YAML (studio/text.js), on cases the examples do not
// all have: each way a formula or an input is written, with comments on the
// line, in flow mappings, and with characters that need quoting. Each case
// gives the whole file and the whole expected file: anything else changing
// fails it, byte for byte.
//
//   node studio/tests/text.mjs

import { setFormulaText, setInputText } from "../text.js";

const FILE = `_forge_version: "5.1.0"
# a comment at the top
year:
  margin:
    formula: "=profit / revenue"   # share of revenue
    unit: ratio
  flow: {formula: "=a + b", unit: CAD}   # flow
  flow_last: {unit: CAD, formula: =a + b}
  plain: =a + b # plain
  single: '=IF(a, "y", "n")'
  price: 16   # dollars
  hours:
    better: lower
    value: 32   # per day
  rate: {value: 0.03, unit: ratio}
months:
  dozens: "=a * 2"
  profit: =revenue - costs
`;
const failures = [];
const lines = (text) => text.split("\n");
function expectLine(what, actual, lineNo, expectedLine) {
  if (actual === null) { failures.push(`${what}: not found`); return; }
  const want = lines(FILE);
  want[lineNo] = expectedLine;
  const got = lines(actual);
  if (got.join("\n") !== want.join("\n")) {
    const i = got.findIndex((l, k) => l !== want[k]);
    failures.push(`${what}: line ${i + 1}\n    expected: ${want[i]}\n    got:      ${got[i]}`);
  }
}
const at = (needle) => lines(FILE).findIndex((l) => l.includes(needle));

// Formulas: the style of the file is kept, and so is everything after the value
expectLine("block formula", setFormulaText(FILE, "year", "margin", "=profit / revenue * 100"), at('formula: "=profit'),
  '    formula: "=profit / revenue * 100"   # share of revenue');
expectLine("flow formula", setFormulaText(FILE, "year", "flow", "=(a + b)"), at("flow: {"),
  '  flow: {formula: "=(a + b)", unit: CAD}   # flow');
expectLine("flow formula, last key, plain", setFormulaText(FILE, "year", "flow_last", "=a * b"), at("flow_last"),
  "  flow_last: {unit: CAD, formula: =a * b}");
expectLine("flow formula, plain, needs quotes", setFormulaText(FILE, "year", "flow_last", "=MAX(a, b)"), at("flow_last"),
  '  flow_last: {unit: CAD, formula: "=MAX(a, b)"}');
expectLine("plain formula", setFormulaText(FILE, "year", "plain", "=a - b"), at("plain: ="),
  "  plain: =a - b # plain");
expectLine("plain formula, needs quotes", setFormulaText(FILE, "year", "plain", '=IF(a, "x", "y")'), at("plain: ="),
  '  plain: "=IF(a, \\"x\\", \\"y\\")" # plain');
expectLine("plain formula with a # in it", setFormulaText(FILE, "year", "plain", "=a #b"), at("plain: ="),
  '  plain: "=a #b" # plain');
expectLine("single-quoted formula", setFormulaText(FILE, "year", "single", `=IF(a, "it's", "n")`), at("single:"),
  `  single: '=IF(a, "it''s", "n")'`);
expectLine("double-quoted formula, backslash", setFormulaText(FILE, "months", "dozens", '=a & "\\"'), at("dozens:"),
  '  dozens: "=a & \\"\\\\\\""');
expectLine("column, same name as a scalar elsewhere", setFormulaText(FILE, "months", "profit", "=revenue - costs - 1"), at("profit: =revenue"),
  "  profit: =revenue - costs - 1");
if (setFormulaText(FILE, "year", "nothing", "=1") !== null) failures.push("a missing name must give null");
if (setFormulaText(FILE, "year", "price", "=1") !== null) failures.push("an input must not become a formula");
if (setFormulaText(FILE, "year", "margin", "=a\nb") !== null) failures.push("a formula on two lines must give null");

// Inputs, as before S7
expectLine("input", setInputText(FILE, "year", "price", 17), at("price:"), "  price: 17   # dollars");
expectLine("input under value:", setInputText(FILE, "year", "hours", 30), at("value: 32"), "    value: 30   # per day");
expectLine("input in a flow mapping", setInputText(FILE, "year", "rate", 0.04), at("rate:"), "  rate: {value: 0.04, unit: ratio}");

if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log("Text changes: 13 formula and 3 input cases change their one value and nothing else.");
