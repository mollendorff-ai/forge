// Studio's text changes to YAML (studio/text.js), on cases the examples do not
// all have: each way a formula or an input is written, with comments on the
// line, in flow mappings, and with characters that need quoting. Each case
// gives the whole file and the whole expected file: anything else changing
// fails it, byte for byte.
//
//   node studio/tests/text.mjs

import { setFormulaText, setInputText, addEntryText, deleteEntryText } from "../text.js";

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
  tiny: 1e-5   # an exponent
  loss: -3
  hex: 0x10
  small:
    value: 1.5E+3
  flow_exp: {value: 2.5e-4, unit: ratio}
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

expectLine("input written 1e-5", setInputText(FILE, "year", "tiny", 0.00002), at("tiny:"), "  tiny: 0.00002   # an exponent");
expectLine("input written -3", setInputText(FILE, "year", "loss", -4), at("loss:"), "  loss: -4");
expectLine("input written in hex", setInputText(FILE, "year", "hex", 17), at("hex:"), "  hex: 17");
expectLine("input under value:, with an exponent", setInputText(FILE, "year", "small", 1600), at("value: 1.5E+3"), "    value: 1600");
expectLine("input in a flow mapping, with an exponent", setInputText(FILE, "year", "flow_exp", 1e-7), at("flow_exp:"), "  flow_exp: {value: 1e-7, unit: ratio}");
if (setInputText(FILE, "year", "nothing", 1) !== null) failures.push("an input that is not there must give null");
if (setInputText(FILE, "year", "margin", 1) !== null) failures.push("a formula must not become an input");

// Adding and deleting an element (S9): the new lines go at the end of the
// group, in the indentation of its keys, and deleting them gives back the
// file byte for byte
const TOP = `_forge_version: "5.1.0"
rate: 0.05   # top level
growth:
  formula: "=rate * 2"

# a comment before the group
year:
  margin:
    formula: "=profit / revenue"   # share of revenue
  price: 16
  hours:
    value: 32

months:
  month: [1, 2, 3]
  # a comment above a column
  sales: "=month * 10"
  list:
  - 1
  - 2
  - 3
four:
    deep: 1
    other: 2
`;
let added = 0, deleted = 0;
function expectAdd(what, group, name, entry, newLines, afterLine) {
  const got = addEntryText(TOP, group, name, entry);
  if (got === null) { failures.push(`${what}: not added`); return; }
  const want = lines(TOP);
  want.splice(lines(TOP).findIndex((l) => l === afterLine) + 1, 0, ...newLines);
  if (got !== want.join("\n")) {
    const g = lines(got), i = g.findIndex((l, k) => l !== want[k]);
    failures.push(`${what}: line ${i + 1}\n    expected: ${want[i]}\n    got:      ${g[i]}`);
    return;
  }
  added++;
  const back = deleteEntryText(got, group, name);
  if (back !== TOP) failures.push(`${what}: deleting it again did not give back the file`);
  else deleted++;
}
expectAdd("input", "year", "tax", { kind: "input", value: 0.15 }, ["  tax: 0.15"], "    value: 32");
expectAdd("input with a unit", "year", "fee", { kind: "input", value: 1e-7, unit: "CAD" }, ["  fee:", "    value: 1e-7", "    unit: CAD"], "    value: 32");
expectAdd("input with a unit that needs quotes", "year", "share", { kind: "input", value: -3, unit: "%" }, ["  share:", "    value: -3", '    unit: "%"'], "    value: 32");
expectAdd("formula", "year", "profit_share", { kind: "formula", formula: "=profit / 2" }, ["  profit_share:", '    formula: "=profit / 2"'], "    value: 32");
expectAdd("formula with quotes and a unit", "year", "label", { kind: "formula", formula: '=IF(a, "x", "y")', unit: "text" }, ["  label:", '    formula: "=IF(a, \\"x\\", \\"y\\")"', "    unit: text"], "    value: 32");
expectAdd("column after a block sequence", "months", "double", { kind: "column", formula: "=sales * 2" }, ['  double: "=sales * 2"'], "  - 3");
expectAdd("group indented by 4", "four", "more", { kind: "input", value: 3 }, ["    more: 3"], "    other: 2");
expectAdd("top level, after a scalar", null, "tax", { kind: "input", value: 0.2, after: "rate" }, ["tax: 0.2"], "rate: 0.05   # top level");
expectAdd("top level, after a formula", null, "x", { kind: "formula", formula: "=rate", after: "growth" }, ["x:", '  formula: "=rate"'], '  formula: "=rate * 2"');

// Deleting what the file already has: its lines go, comments above stay
const expectDelete = (what, group, name, from, to) => {
  const got = deleteEntryText(TOP, group, name);
  const want = lines(TOP).filter((_, k) => k < from || k > to).join("\n");
  if (got !== want) failures.push(`${what}: ${got === null ? "not deleted" : "other lines changed"}`);
  else deleted++;
};
const L = (needle) => lines(TOP).findIndex((l) => l.includes(needle));
expectDelete("a formula mapping with a comment", "year", "margin", L("margin:"), L("share of revenue"));
expectDelete("an input under value:", "year", "hours", L("hours:"), L("value: 32"));
expectDelete("a column, keeping the comment above it", "months", "sales", L("sales:"), L("sales:"));
expectDelete("a block sequence column", "months", "list", L("list:"), L("- 3"));
expectDelete("a top-level scalar with a comment", null, "rate", L("rate:"), L("rate:"));

// Refused, so the caller can say so
const refuse = (what, got) => { if (got !== null) failures.push(`${what} must give null`); };
refuse("an existing name", addEntryText(TOP, "year", "price", { kind: "input", value: 1 }));
refuse("a name YAML would misread", addEntryText(TOP, "year", "a b", { kind: "input", value: 1 }));
refuse("a group that is not there", addEntryText(TOP, "nothing", "x", { kind: "input", value: 1 }));
refuse("a formula without =", addEntryText(TOP, "year", "x", { kind: "formula", formula: "a + b" }));
refuse("a formula on two lines", addEntryText(TOP, "year", "x", { kind: "formula", formula: "=a\n+ b" }));
refuse("an input that is not a number", addEntryText(TOP, "year", "x", { kind: "input", value: NaN }));
refuse("the top level with nothing to follow", addEntryText(TOP, null, "x", { kind: "input", value: 1 }));
refuse("a column at the top level", addEntryText(TOP, null, "x", { kind: "column", formula: "=1", after: "rate" }));
refuse("deleting a name that is not there", deleteEntryText(TOP, "year", "nothing"));
refuse("deleting the last element of a group", deleteEntryText(addEntryText(TOP, null, "solo", { kind: "input", value: 1, after: "growth" }).replace("solo: 1", "solo_group:\n  only: 1"), "solo_group", "only"));

if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log(`Text changes: 13 formula and 10 input cases change their one value and nothing else; ${added} elements are added in the file's style, and ${deleted} deletions remove exactly their lines.`);
