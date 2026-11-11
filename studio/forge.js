// The page's only way to a number: forge-wasm, running Forge's own core
// functions in a worker (ADR-055, ADR-056). Every tool answers a promise of
// the result, or rejects with Forge's message.

let worker = null;
let next = 1;
const waiting = new Map();
let started = null;

export function start() {
  if (started) return started;
  started = new Promise((resolve, reject) => {
    if (typeof WebAssembly !== "object") {
      reject(new Error("This browser cannot run WebAssembly, which Forge Studio needs to run Forge."));
      return;
    }
    try {
      worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });
    } catch (e) {
      reject(new Error("This browser cannot start Forge in the background (module workers)."));
      return;
    }
    worker.onmessage = ({ data }) => {
      if (data.ready) return resolve(data.version);
      if (data.failed) return reject(new Error(`Forge could not load: ${data.failed}`));
      const w = waiting.get(data.id);
      if (!w) return;
      waiting.delete(data.id);
      const answer = JSON.parse(data.answer);
      if ("error" in answer) w.reject(new Error(answer.error));
      else w.resolve(answer.ok);
    };
    worker.onerror = (e) => reject(new Error(`Forge could not load: ${e.message || "the worker failed"}`));
  });
  return started;
}

export async function call(tool, request) {
  await start();
  const id = next++;
  return new Promise((resolve, reject) => {
    waiting.set(id, { resolve, reject });
    worker.postMessage({ id, tool, request: JSON.stringify(request) });
  });
}
