// Tokenizes every YAML file of the repository with VS Code's own YAML grammar,
// with and without the formula injection, and checks that:
// - nothing outside a formula changes;
// - every line that looks like a formula ("=...", '=...', key: =...) is one;
// - every piece of a formula gets a Forge scope;
// - a few formulas get exactly the scopes expected.
//
// VS Code's YAML grammar is downloaded once from a pinned commit of
// microsoft/vscode, or read from $YAML_GRAMMARS.

const fs = require('fs');
const path = require('path');
const tm = require('vscode-textmate');
const onig = require('vscode-oniguruma');

const VSCODE_COMMIT = 'a247f16f7b7f6e4d56ecf883b442763e19eec096';
const GRAMMARS = ['yaml', 'yaml-1.0', 'yaml-1.1', 'yaml-1.2', 'yaml-1.3', 'yaml-embedded'];
const root = path.resolve(__dirname, '../../..');

async function yamlGrammars() {
  const dir = process.env.YAML_GRAMMARS || path.join(__dirname, '../.cache', VSCODE_COMMIT);
  fs.mkdirSync(dir, { recursive: true });
  const out = {};
  for (const name of GRAMMARS) {
    const file = path.join(dir, `${name}.tmLanguage.json`);
    if (!fs.existsSync(file)) {
      const url = `https://raw.githubusercontent.com/microsoft/vscode/${VSCODE_COMMIT}/extensions/yaml/syntaxes/${name}.tmLanguage.json`;
      const res = await fetch(url);
      if (!res.ok) throw new Error(`${url}: ${res.status}`);
      fs.writeFileSync(file, await res.text());
    }
    const g = JSON.parse(fs.readFileSync(file, 'utf8'));
    out[g.scopeName] = g;
  }
  return out;
}

async function grammar(withInjection, files) {
  const wasm = fs.readFileSync(require.resolve('vscode-oniguruma/release/onig.wasm')).buffer;
  await onig.loadWASM(wasm).catch(() => {}); // already loaded on the second call
  const manifest = require('../package.json');
  const injections = manifest.contributes.grammars;
  const all = { ...files };
  for (const injection of injections) {
    all[injection.scopeName] = JSON.parse(fs.readFileSync(path.join(__dirname, '..', injection.path), 'utf8'));
  }
  const registry = new tm.Registry({
    onigLib: Promise.resolve({
      createOnigScanner: (p) => new onig.OnigScanner(p),
      createOnigString: (s) => new onig.OnigString(s),
    }),
    loadGrammar: async (scope) => (all[scope] ? tm.parseRawGrammar(JSON.stringify(all[scope]), 'g.json') : null),
    getInjections: (scope) =>
      withInjection ? injections.filter((i) => i.injectTo.includes(scope)).map((i) => i.scopeName) : [],
  });
  return registry.loadGrammar('source.yaml');
}

function tokenize(g, text) {
  let state = tm.INITIAL;
  return text.split('\n').map((line) => {
    const r = g.tokenizeLine(line, state);
    state = r.ruleStack;
    return r.tokens.map((t) => ({ text: line.slice(t.startIndex, t.endIndex), scopes: t.scopes }));
  });
}

// Scopes of each character of a line
const perChar = (tokens, dropForge) =>
  tokens.flatMap((t) => Array(t.text.length).fill(t.scopes.filter((s) => !(dropForge && s.endsWith('.forge'))).join(' ')));
const inFormula = (scopes) => scopes.includes('meta.formula.forge') || scopes.includes('keyword.operator.formula.forge');

function yamlFiles(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = path.join(dir, e.name);
    if (e.isDirectory()) return ['target', 'node_modules', '.git', '.cache'].includes(e.name) ? [] : yamlFiles(p);
    return /\.ya?ml$/.test(e.name) ? [p] : [];
  });
}

(async () => {
  const files = await yamlGrammars();
  const plain = await grammar(false, files);
  const forge = await grammar(true, files);
  const failures = [];
  let formulas = 0;
  const models = yamlFiles(root);
  for (const file of models) {
    const lines = fs.readFileSync(file, 'utf8').split('\n');
    const before = tokenize(plain, lines.join('\n'));
    const after = tokenize(forge, lines.join('\n'));
    lines.forEach((line, i) => {
      const where = `${path.relative(root, file)}:${i + 1}`;
      const a = perChar(after[i], false);
      const inside = a.map(inFormula);
      const b0 = perChar(before[i], false).filter((_, k) => !inside[k]);
      const a0 = perChar(after[i], true).filter((_, k) => !inside[k]);
      if (JSON.stringify(b0) !== JSON.stringify(a0)) failures.push(`${where}: changed outside a formula: ${line}`);
      const found = inside.some(Boolean);
      // A value that starts with "=": after `key: ` or `- ` at the start of
      // the line, or an entry of a flow list
      const looks =
        !line.trimStart().startsWith('#') &&
        (/^\s*(?:-\s+)?(?:[\w.@-]+:\s+)?["']?=[^=]/.test(line) || /\[\s*["']?=[^=]|,\s*["']=[^=]/.test(line));
      if (found) formulas += 1;
      if (looks && !found) failures.push(`${where}: formula not recognised: ${line}`);
      for (const t of after[i]) {
        if (t.text.trim() && t.scopes.includes('meta.formula.forge') && !t.scopes.some((s) => s.endsWith('.forge') && s !== 'meta.formula.forge')) {
          failures.push(`${where}: '${t.text}' has no Forge scope: ${line}`);
        }
      }
    });
  }

  // Exact scopes for a few formulas
  const expect = [
    ['  x: "=a + SUM(t.x) * 2"', [['=', 'keyword.operator.formula'], ['a', 'variable.other'], ['+', 'keyword.operator'], ['SUM', 'support.function'], ['(', 'punctuation.separator'], ['t.x', 'variable.other'], [')', 'punctuation.separator'], ['*', 'keyword.operator'], ['2', 'constant.numeric']]],
    ['  y: =loan.principal / 12', [['=', 'keyword.operator.formula'], ['loan.principal', 'variable.other'], ['/', 'keyword.operator'], ['12', 'constant.numeric']]],
    ["  w: '=MC.Normal(1, 2.5)'", [['=', 'keyword.operator.formula'], ['MC.Normal', 'support.function'], ['(', 'punctuation.separator'], ['1', 'constant.numeric'], [',', 'punctuation.separator'], ['2.5', 'constant.numeric'], [')', 'punctuation.separator']]],
    ['  f: "=@costs.flour_per_kg ^ (1 / 6)"', [['=', 'keyword.operator.formula'], ['@costs.flour_per_kg', 'variable.other.include'], ['^', 'keyword.operator'], ['(', 'punctuation.separator'], ['1', 'constant.numeric'], ['/', 'keyword.operator'], ['6', 'constant.numeric'], [')', 'punctuation.separator']]],
    ['  s: "=SUMIF(r, \\"North\\", v)"', [['=', 'keyword.operator.formula'], ['SUMIF', 'support.function'], ['(', 'punctuation.separator'], ['r', 'variable.other'], [',', 'punctuation.separator'], ['\\"North\\"', 'string.quoted.double.formula'], [',', 'punctuation.separator'], ['v', 'variable.other'], [')', 'punctuation.separator']]],
    ['  t: \'=IF(x >= 0, "yes", TRUE)\'', [['=', 'keyword.operator.formula'], ['IF', 'support.function'], ['(', 'punctuation.separator'], ['x', 'variable.other'], ['>=', 'keyword.operator'], ['0', 'constant.numeric'], [',', 'punctuation.separator'], ['"yes"', 'string.quoted.double.formula'], [',', 'punctuation.separator'], ['TRUE', 'constant.language.boolean'], [')', 'punctuation.separator']]],
    ['  note: "a = b"', []],
    // Text that only contains ": =" is not a formula, in any kind of string
    ['  why: "found in cases: =SUM(a, b) fails"', []],
    ["  why: 'found in cases: =SUM(a, b) fails'", []],
    ['  why: "say \\"=x\\" here"', []],
    ["  why: 'it''=s'", []],
    ['  - =a * 2', [['=', 'keyword.operator.formula'], ['a', 'variable.other'], ['*', 'keyword.operator'], ['2', 'constant.numeric']]],
  ];
  for (const [line, want] of expect) {
    const got = tokenize(forge, `a:\n${line}`)[1]
      .filter((t) => t.text.trim() && t.scopes.some((s) => s.endsWith('.forge') && s !== 'meta.formula.forge'))
      .map((t) => [t.text.trim(), t.scopes[t.scopes.length - 1].replace(/\.forge$/, '')]);
    if (JSON.stringify(got) !== JSON.stringify(want)) failures.push(`${line}\n    expected ${JSON.stringify(want)}\n    got      ${JSON.stringify(got)}`);
  }

  console.log(`${models.length} YAML files, ${formulas} formula lines, ${expect.length} exact checks`);
  if (failures.length) {
    console.error(failures.join('\n'));
    process.exit(1);
  }
  console.log('grammar: ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
