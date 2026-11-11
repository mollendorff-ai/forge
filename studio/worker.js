// Forge in a background worker, so a long simulation does not freeze the page.
// Messages in: {id, tool, request}; out: {id, answer} with the JSON text that
// forge-wasm's call() returns, or {ready, version} once, or {failed} if the
// WebAssembly module cannot load.

import init, { call, version } from "./pkg/forge_wasm.js";

const ready = init()
  .then(() => postMessage({ ready: true, version: version() }))
  .catch((e) => postMessage({ failed: String(e && e.message ? e.message : e) }));

onmessage = async ({ data }) => {
  await ready;
  let answer;
  try {
    answer = call(data.tool, data.request);
  } catch (e) {
    answer = JSON.stringify({ error: `Forge stopped: ${e && e.message ? e.message : e}` });
  }
  postMessage({ id: data.id, answer });
};
