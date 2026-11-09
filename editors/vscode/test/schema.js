// Checks the schema the extension bundles with yaml-language-server, the
// language server behind VS Code's YAML extension (and Zed's YAML support):
// - the bundled schema is the repository's current one;
// - every example model gets no diagnostic;
// - a model with mistakes gets one on each mistaken line, and none on the
//   valid line beside them.

const { spawn } = require('child_process');
const fs = require('fs');
const path = require('path');

const root = path.resolve(__dirname, '../../..');
const manifest = require('../package.json');
const bundled = path.join(__dirname, '..', manifest.contributes.yamlValidation[0].url);
const source = path.join(root, 'schema', path.basename(bundled));

function examples(dir) {
  return fs.readdirSync(dir, { withFileTypes: true }).flatMap((e) => {
    const p = path.join(dir, e.name);
    return e.isDirectory() ? examples(p) : /\.ya?ml$/.test(e.name) ? [p] : [];
  });
}

const BAD = [
  '_forge_version: "5.1.0"',
  'loan:',
  '  principal: 450000',
  '  payment:',
  '    formla: "=principal * 2"',
  '  rate:',
  '    value: "high"',
  '',
].join('\n');

function validate(docs) {
  const server = spawn(process.execPath, [require.resolve('yaml-language-server/bin/yaml-language-server'), '--stdio']);
  const settings = { validate: true, schemaStore: { enable: false }, schemas: { [`file://${bundled}`]: '*.yaml' } };
  const diagnostics = {};
  let buffer = Buffer.alloc(0);
  let id = 0;
  const pending = {};
  const send = (m) => {
    const s = JSON.stringify({ jsonrpc: '2.0', ...m });
    server.stdin.write(`Content-Length: ${Buffer.byteLength(s)}\r\n\r\n${s}`);
  };
  const request = (method, params) => new Promise((resolve) => { pending[++id] = resolve; send({ id, method, params }); });
  server.stdout.on('data', (data) => {
    buffer = Buffer.concat([buffer, data]);
    for (;;) {
      const end = buffer.indexOf('\r\n\r\n');
      if (end < 0) return;
      const length = Number(/Content-Length: (\d+)/.exec(buffer.slice(0, end).toString())[1]);
      if (buffer.length < end + 4 + length) return;
      const m = JSON.parse(buffer.slice(end + 4, end + 4 + length));
      buffer = buffer.slice(end + 4 + length);
      if (m.id !== undefined && pending[m.id] && !m.method) pending[m.id](m.result);
      else if (m.method === 'textDocument/publishDiagnostics') diagnostics[m.params.uri] = m.params.diagnostics;
      else if (m.method === 'workspace/configuration') send({ id: m.id, result: m.params.items.map(() => settings) });
      else if (m.id !== undefined && m.method) send({ id: m.id, result: null });
    }
  });
  return (async () => {
    await request('initialize', { processId: null, rootUri: null, capabilities: { workspace: { configuration: true } } });
    send({ method: 'initialized', params: {} });
    send({ method: 'workspace/didChangeConfiguration', params: { settings: { yaml: settings } } });
    for (const [uri, text] of docs) {
      send({ method: 'textDocument/didOpen', params: { textDocument: { uri, languageId: 'yaml', version: 1, text } } });
    }
    const deadline = Date.now() + 60000;
    while (Object.keys(diagnostics).length < docs.length && Date.now() < deadline) {
      await new Promise((r) => setTimeout(r, 200));
    }
    // Diagnostics can be published twice; wait for the last
    await new Promise((r) => setTimeout(r, 1500));
    server.kill();
    return diagnostics;
  })();
}

(async () => {
  const failures = [];
  if (fs.readFileSync(bundled, 'utf8') !== fs.readFileSync(source, 'utf8')) {
    failures.push(`${path.relative(root, bundled)} differs from ${path.relative(root, source)}: copy it again`);
  }
  const files = examples(path.join(root, 'examples'));
  const docs = files.map((f) => [`file://${f}`, fs.readFileSync(f, 'utf8')]);
  docs.push(['file:///tmp/forge-bad.yaml', BAD]);
  const found = await validate(docs);
  for (const [uri] of docs.slice(0, -1)) {
    const d = found[uri];
    if (d === undefined) failures.push(`${uri}: no diagnostics published`);
    else if (d.length) failures.push(`${uri}: ${d.map((x) => `line ${x.range.start.line + 1}: ${x.message}`).join('; ')}`);
  }
  const bad = found['file:///tmp/forge-bad.yaml'] || [];
  const lines = new Set(bad.map((x) => x.range.start.line + 1));
  for (const line of [5, 7]) if (!lines.has(line)) failures.push(`the misspelt key and the text value (line ${line}) got no diagnostic: ${JSON.stringify(bad)}`);
  // The schema reads a group as the parser does, so the valid scalar beside
  // the mistakes is not marked
  if (lines.has(3)) failures.push(`the valid line 3 (principal) got a diagnostic: ${JSON.stringify(bad)}`);

  console.log(`${files.length} examples validated against ${path.basename(bundled)}, and one model with mistakes`);
  if (failures.length) {
    console.error(failures.join('\n'));
    process.exit(1);
  }
  console.log('schema: ok');
})().catch((e) => {
  console.error(e);
  process.exit(1);
});
