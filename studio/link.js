// A link that carries a model after the # (ADR-080). The part of an address
// after # is never sent to a server, so a model opened from a link stays in
// the tab, as one opened from the device does.
//
//   #name=bagel.yaml&gzip=…&name=costs.yaml&yaml=…
//
// `name=` names the next file (model.yaml without one); `yaml=` is its text,
// percent-encoded, as a person or an AI writes it by hand; `gzip=` is its
// text gzipped and in base64url, as Studio writes it. The first file is the
// main model.

import { esc } from "./text.js";

const KEYS = new Set(["name", "yaml", "gzip"]);
// More than this, once decoded, is not a model a link should carry
export const MOST = 2_000_000;
const FILE = /^[^/\\]+\.ya?ml$/i;

// Whether a fragment is a model at all: #an-sim, inside the page, is not
export const isModelLink = (hash) => hash.replace(/^#/, "").split("&").some((part) => KEYS.has(part.split("=")[0]));

export const toBase64 = (bytes) => {
  let text = "";
  for (let i = 0; i < bytes.length; i += 0x8000) text += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
  return btoa(text);
};
export const fromBase64 = (b64) => Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
const toBase64url = (bytes) => toBase64(bytes).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
const fromBase64url = (text) => {
  const b64 = text.replace(/-/g, "+").replace(/_/g, "/") + "=".repeat((4 - (text.length % 4)) % 4);
  try { if (/^[A-Za-z0-9_-]*$/.test(text)) return fromBase64(b64); } catch (e) { /* a length no base64 has */ }
  throw new Error("its gzip= part is not base64url");
};

// Gunzips, stopping once the text is past MOST, so that a small link cannot
// unpack into more than the tab can hold
async function gunzip(bytes, room) {
  const reader = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip")).getReader();
  const chunks = [];
  let size = 0;
  for (;;) {
    let step;
    try { step = await reader.read(); } catch (e) { throw new Error("its gzip= part is not gzip"); }
    if (step.done) break;
    size += step.value.length;
    if (size > room) { await reader.cancel(); throw tooLarge(); }
    chunks.push(step.value);
  }
  try { return new TextDecoder("utf-8", { fatal: true }).decode(await new Blob(chunks).arrayBuffer()); } catch (e) { throw new Error("its text is not UTF-8"); }
}
const tooLarge = () => new Error(`its text is more than ${MOST.toLocaleString("en")} bytes`);
const decode = (text, what) => {
  try { return decodeURIComponent(text); } catch (e) { throw new Error(`its ${what} is not percent-encoded`); }
};

// The files a fragment carries, as { files, main }; throws with what is wrong
export async function readLink(hash) {
  const files = {};
  let main = null;
  let name = null;
  let size = 0;
  for (const part of hash.replace(/^#/, "").split("&")) {
    if (!part) continue;
    const at = part.indexOf("=");
    const key = at < 0 ? part : part.slice(0, at);
    const value = at < 0 ? "" : part.slice(at + 1);
    if (!KEYS.has(key)) throw new Error(`it has "${key}", which Forge Studio does not read`);
    if (key === "name") {
      if (name !== null) throw new Error(`the name ${name} has no file after it`);
      name = decode(value, "name");
      if (!FILE.test(name)) throw new Error(`${name} is not the name of a YAML file (a name ending in .yaml or .yml, with no folder)`);
      continue;
    }
    const file = name ?? "model.yaml";
    name = null;
    if (Object.hasOwn(files, file)) throw new Error(`it has two files named ${file}`);
    const text = key === "yaml" ? decode(value, "yaml= part") : await gunzip(fromBase64url(value), MOST - size);
    size += new TextEncoder().encode(text).length;
    if (size > MOST) throw tooLarge();
    files[file] = text;
    main ??= file;
  }
  if (name !== null) throw new Error(`the name ${name} has no file after it`);
  if (main === null) throw new Error("it has no file");
  return { files, main };
}

// The fragment for these files, the main one first, in the gzip= form
export async function makeLink(files, order) {
  const parts = [];
  for (const file of order) {
    const zipped = new Blob([files[file]]).stream().pipeThrough(new CompressionStream("gzip"));
    parts.push(`name=${encodeURIComponent(file)}`, `gzip=${toBase64url(new Uint8Array(await new Response(zipped).arrayBuffer()))}`);
  }
  return "#" + parts.join("&");
}

// Files' card for the link to the open model
export const linkCard = (names) => `<div class="eyebrow"><h2>link</h2><span class="tag">the model after the #</span></div>
    <div class="card row form"><div class="file"><div><b>A link to ${esc(names.join(" + "))}</b><p class="note">The model is in the link itself, after the #, which no server receives. Anyone who has the link has the model, and the link stays in the history of the browser that opens it.</p></div>
      <button class="btn primary" type="button" id="make-link">Copy</button></div><div id="link-msg"></div></div>`;

// Opening a link, and making one. `app` gives what is open and how to open
// a link's files:
//   replaced()          the open files a link would replace that are kept only here
//   sheet(title, html)  shows a message
//   open(link, now)     opens the link's files, and draws them when `now`
//   model()             { files, order } of the open model, its main file first
export function listenLinks(app) {
  let waiting = null;
  let made = null;
  const sheet = () => document.querySelector("#sheet");
  // Opens the model in a fragment, or asks first when it would replace the
  // person's own work; answers whether it is open. The fragment leaves the
  // address bar either way, the query with it, so a reload shows the model
  // as last edited.
  async function open(hash, now) {
    history.replaceState(history.state, "", location.pathname);
    let link;
    try {
      link = await readLink(hash);
    } catch (e) {
      app.sheet("This link could not be opened", `<p class="note">${esc(`The model in this link could not be read: ${e.message}. What was open is still open.`)}</p>`);
      return false;
    }
    const replaced = app.replaced();
    if (replaced.length) {
      waiting = link;
      app.sheet("Open the link's model?", `<p class="note">${esc(`The link opens ${Object.keys(link.files).join(" + ")}. It replaces ${replaced.join(", ")}, kept only in this browser: Save on the Files tab keeps a copy.`)}</p>
        <div class="btns"><button class="btn primary" type="button" id="link-open">Open the link</button><button class="btn" type="button" id="link-keep">Keep my files</button></div>`);
      return false;
    }
    await app.open(link, now);
    return true;
  }
  // Made when Files is drawn, so that a tap on Copy writes to the clipboard
  // at once: Safari allows it only within the tap
  async function prepare() {
    const { files, order } = app.model();
    const key = JSON.stringify([order, files]);
    if (!made || made.key !== key) made = { key, url: location.origin + location.pathname + (await makeLink(files, order)) };
    return made;
  }
  async function copy() {
    const { url } = await prepare();
    let copied = false;
    try { await navigator.clipboard.writeText(url); copied = true; } catch (e) { /* not allowed here: the field below */ }
    document.querySelector("#link-msg").innerHTML = `<p class="note" style="margin-top:8px">${copied ? "Copied" : "Copy it from here"}: ${url.length.toLocaleString("en")} characters.</p><input type="text" class="mono" id="link-text" readonly aria-label="The link" value="${esc(url)}">`;
    if (!copied) document.querySelector("#link-text").select();
  }
  // A link pasted into the address bar of Studio already open
  window.addEventListener("hashchange", () => { if (isModelLink(location.hash)) open(location.hash, true); });
  document.addEventListener("click", async (e) => {
    if (e.target.id === "make-link") await copy();
    if (e.target.id === "link-keep") { waiting = null; sheet().hidden = true; }
    if (e.target.id === "link-open" && waiting) {
      const link = waiting;
      waiting = null;
      sheet().hidden = true;
      await app.open(link, true);
    }
  });
  return { open, prepare };
}
