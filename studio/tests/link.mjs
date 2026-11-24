// A link that carries a model after the # (ADR-080), in a headless browser:
//
// - the mortgage, the bagel shop with its costs (two files) and the starter
//   model open from links, in the gzip= and the yaml= forms, made here with
//   Node's zlib as the `gzip` tool would, and every scalar on screen is
//   `forge calculate`'s (forge-wasm/expected.json);
// - the link leaves the address bar once read, and a reload shows the model;
// - a link, or ?example=, over the person's own files asks first: Keep my files keeps them,
//   Open the link opens it; a link pasted into an open tab opens too;
// - a broken link says what is wrong and keeps what was open; #an-sim is
//   not a model and changes nothing;
// - Files' Copy makes a link that opens the same model in another browser;
// - text a link carries is drawn as text: no element and no script of its own;
// - no request to the server carries any part of a model, and the page's
//   Content Security Policy blocks nothing Studio uses.
//
//   node studio/tests/link.mjs <built Studio dir> <expected.json>

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync } from "node:fs";
import { join, extname, resolve } from "node:path";
import { gzipSync } from "node:zlib";
import { chromium } from "playwright";

const [where, expectedPath] = process.argv.slice(2);
const dir = resolve(where);
const expected = JSON.parse(readFileSync(resolve(expectedPath), "utf8"));
const examples = new URL("../../examples/", import.meta.url);
const example = (path) => readFileSync(new URL(path, examples), "utf8");
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".wasm": "application/wasm" };
// Every request the server sees: its address and its headers
const requests = [];
const server = createServer((req, res) => {
  requests.push(JSON.stringify([req.url, req.headers]));
  let file = join(dir, decodeURIComponent(new URL(req.url, "http://x").pathname));
  if (existsSync(file) && statSync(file).isDirectory()) file = join(file, "index.html");
  if (!file.startsWith(dir) || !existsSync(file)) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { "content-type": TYPES[extname(file)] || "application/octet-stream" });
  res.end(readFileSync(file));
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const base = `http://127.0.0.1:${server.address().port}/`;

const failures = [];
// While the policy is probed on purpose, the browser's reports of it are expected
let probing = false;
const fail = (m) => failures.push(m);
const RELATIVE = 1e-9;
const close = (a, b) => Math.abs(a - b) <= RELATIVE * Math.max(1, Math.abs(a), Math.abs(b));

// The two forms of ADR-080, as a person, an AI or `gzip | base64` makes them
const gzip = (text) => gzipSync(Buffer.from(text, "utf8"), { level: 9 }).toString("base64url");
const part = (name, text, form) => `${name ? `name=${encodeURIComponent(name)}&` : ""}${form === "yaml" ? `yaml=${encodeURIComponent(text)}` : `gzip=${gzip(text)}`}`;
const MORTGAGE = example("mortgage/mortgage.yaml");
const BAGEL = example("bagel/bagel.yaml");
const COSTS = example("bagel/costs.yaml");
const STARTER = example("starter.yaml");
const links = {
  mortgage: "#" + part("mortgage.yaml", MORTGAGE, "gzip"),
  bagel: "#" + part("bagel.yaml", BAGEL, "gzip") + "&" + part("costs.yaml", COSTS, "yaml"),
  starter: "#" + part(null, STARTER, "yaml"),
};

const browser = await chromium.launch();
async function newPage(context) {
  const page = await context.newPage();
  page.on("pageerror", (e) => fail(`page error: ${e.message}`));
  page.on("console", (m) => { if (m.type() === "error" && !probing) fail(`console error: ${m.text()}`); });
  // What the Content Security Policy blocks, and whether a script of a link's ran
  await page.addInitScript(() => {
    window.blocked = [];
    document.addEventListener("securitypolicyviolation", (e) => window.blocked.push(`${e.violatedDirective} ${e.blockedURI}`));
  });
  return page;
}
const ready = (page) => page.waitForFunction(() => document.body.dataset.state !== "loading" && document.querySelector("[data-scalar]"), null, { timeout: 120000 });
const settle = (page, before) => page.waitForFunction((g) => Number(document.body.dataset.generation) > g && document.body.dataset.state !== "loading", before, { timeout: 120000 });
const generation = (page) => page.evaluate(() => Number(document.body.dataset.generation));
const title = (page) => page.locator("#model-title").innerText();
const sheetTitle = async (page) => ((await page.locator("#sheet").isVisible()) ? page.locator("#sheet-title").innerText() : null);

// Every scalar on screen against forge calculate's answer for the model
async function same(page, label, answer) {
  const native = expected[answer] && expected[answer].ok;
  if (!native) { fail(`${label}: no native answer "${answer}"`); return; }
  const state = await page.evaluate(() => document.body.dataset.state);
  if (state !== "ready") fail(`${label}: the page says ${state}`);
  const seen = new Map(await page.$$eval("[data-scalar]", (els) => els.map((e) => [e.dataset.scalar, e.dataset.value])));
  for (const [path, value] of Object.entries(native.scalars)) {
    if (!seen.has(path)) { fail(`${label}: ${path} is not on screen`); continue; }
    const text = seen.get(path);
    const differs = value === null ? text !== "" : typeof value === "string" ? text !== value : !close(Number(text), value);
    if (differs) fail(`${label}: ${path} shows ${text}, forge calculate gives ${value}`);
  }
  for (const path of seen.keys()) if (!(path in native.scalars)) fail(`${label}: ${path} is on screen but not in forge calculate's answer`);
  if (seen.size < 5) fail(`${label}: only ${seen.size} scalars on screen`);
}
// The address bar keeps no model once the link is read
async function clean(page, label) {
  const url = page.url();
  if (url.includes("#") || url.includes("?")) fail(`${label}: the address bar still has ${url}`);
}

// A link opens over an example as it ships, without asking
const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
const page = await newPage(context);
await page.goto(base + links.mortgage);
await ready(page);
await same(page, "the mortgage's link", "calculate mortgage");
if ((await title(page)) !== "mortgage.yaml") fail(`the mortgage's link opens as ${await title(page)}`);
await clean(page, "the mortgage's link");
if (await sheetTitle(page)) fail(`the mortgage's link over an example asks "${await sheetTitle(page)}"`);

// A reload shows the model, kept in this browser, not the link
await page.reload();
await ready(page);
await same(page, "the mortgage after a reload", "calculate mortgage");

// A link over the person's own files asks first
let before = await generation(page);
await page.evaluate((hash) => { location.hash = hash; }, links.starter);
await page.locator("#link-keep").waitFor();
if ((await sheetTitle(page)) !== "Open the link's model?") fail(`a link over the person's files asks "${await sheetTitle(page)}"`);
if (!/model\.yaml.*replaces mortgage\.yaml/.test(await page.locator("#sheet-body").innerText())) fail("the question does not name the file the link opens and the one it replaces");
await clean(page, "a link not opened yet");
await page.click("#link-keep");
if ((await title(page)) !== "mortgage.yaml" || (await generation(page)) !== before) fail("Keep my files does not keep the mortgage as it was");
await page.evaluate((hash) => { location.hash = hash; }, links.starter);
await page.locator("#link-open").waitFor();
before = await generation(page);
await page.click("#link-open");
await settle(page, before);
await ready(page);
await same(page, "the starter's link, in the yaml= form", "calculate starter");
if ((await title(page)) !== "model.yaml") fail(`a file with no name= opens as ${await title(page)}, not model.yaml`);

// A broken link says what is wrong and keeps what was open
const BROKEN = [
  ["#gzip=not*base64", /not base64url/],
  ["#gzip=abcde", /not base64url/],
  ["#gzip=" + Buffer.from("plain text").toString("base64url"), /not gzip/],
  ["#name=..%2Fcosts.yaml&yaml=a%3A%201", /not the name of a YAML file/],
  ["#name=a.yaml&yaml=x%3A%201&name=a.yaml&yaml=y%3A%202", /two files named a\.yaml/],
  ["#what=1&yaml=x%3A%201", /"what"/],
  ["#name=a.yaml", /has no file after it/],
  ["#yaml=%E0%A4%A", /not percent-encoded/],
  ["#gzip=" + gzipSync(Buffer.alloc(3_000_000, 32)).toString("base64url"), /more than 2,000,000 bytes/],
];
for (const [hash, why] of BROKEN) {
  await page.evaluate((h) => { location.hash = h; }, hash);
  await page.waitForFunction(() => !document.querySelector("#sheet").hidden, null, { timeout: 30000 });
  const text = await page.locator("#sheet-body").innerText();
  if ((await sheetTitle(page)) !== "This link could not be opened" || !why.test(text)) fail(`${hash.slice(0, 40)}: the sheet says "${text}", not ${why}`);
  await page.click("#sheet-close");
  if ((await title(page)) !== "model.yaml") fail(`${hash.slice(0, 40)}: the starter is no longer open`);
}
await clean(page, "a broken link");
await same(page, "the starter after the broken links", "calculate starter");

// #an-sim, inside the page, is not a model
await page.evaluate(() => { location.hash = "#an-sim"; });
await page.waitForTimeout(300);
if (await sheetTitle(page)) fail(`#an-sim opens a sheet: "${await sheetTitle(page)}"`);
if (!page.url().endsWith("#an-sim")) fail("#an-sim was taken out of the address bar");

// Two files, the main one first, after a query that names an example: the
// example opens first, as it ships, so the link opens over it without asking
const fresh = await browser.newContext({ viewport: { width: 390, height: 844 } });
const two = await newPage(fresh);
await two.goto(base + "?example=dcf" + links.bagel);
await ready(two);
if (await sheetTitle(two)) fail(`a link after ?example= asks "${await sheetTitle(two)}"`);
await same(two, "the bagel's link of two files", "calculate bagel");
await clean(two, "a link with a query");
await two.close();

// ?example= over files of the person's own asks first, as a link does
// (the starter, from a link, is open and kept in this browser)
const asking = await newPage(context);
await asking.goto(base + "?example=dcf");
await asking.locator("#example-keep").waitFor();
if ((await sheetTitle(asking)) !== "Open the example?" || !/replaces model\.yaml/.test(await asking.locator("#sheet-body").innerText())) fail("?example= over the person's files does not ask, naming what it replaces");
await asking.click("#example-keep");
if ((await title(asking)) !== "model.yaml") fail("Keep my files does not keep the starter after ?example=");
await asking.reload();
await ready(asking);
await same(asking, "the starter kept after ?example=", "calculate starter");
await asking.goto(base + "?example=dcf");
await asking.locator("#example-open").waitFor();
before = await generation(asking);
await asking.click("#example-open");
await settle(asking, before);
await ready(asking);
await same(asking, "?example=dcf opened after asking", "calculate dcf");
if ((await title(asking)) !== "Five-year DCF") fail(`?example=dcf opens as ${await title(asking)}`);
await asking.close();

// Files' Copy makes a link that opens the same model elsewhere
const maker = await browser.newContext({ viewport: { width: 390, height: 844 }, permissions: ["clipboard-read", "clipboard-write"] });
const mpage = await newPage(maker);
await mpage.goto(base + "?example=bagel");
await ready(mpage);
await mpage.click('[data-tab="files"]');
await mpage.click("#make-link");
await mpage.locator("#link-text").waitFor();
const made = await mpage.inputValue("#link-text");
if (!/^Copied: [\d,]+ characters\.$/.test(await mpage.locator("#link-msg .note").innerText())) fail(`Copy says "${await mpage.locator("#link-msg .note").innerText()}"`);
if ((await mpage.evaluate(() => navigator.clipboard.readText())) !== made) fail("Copy does not put the link on the clipboard");
if (!made.startsWith(`${base}#name=bagel.yaml&gzip=`) || !made.includes("&name=costs.yaml&gzip=")) fail(`Files makes ${made.slice(0, 80)}…, not the bagel and its costs`);
const elsewhere = await browser.newContext({ viewport: { width: 390, height: 844 } });
const epage = await newPage(elsewhere);
await epage.goto(made);
await ready(epage);
await same(epage, "the link Files makes", "calculate bagel");

// Text a link carries is drawn as text
const sneaky = STARTER.replace('"Basic"', '"<img src=x onerror=window.pwned=1>"').replace("# A first Forge model.", "# <script>window.pwned=2</script>");
await epage.goto(base + "#" + part("sneaky.yaml", sneaky, "gzip"));
await epage.locator("#link-open").waitFor();
before = await generation(epage);
await epage.click("#link-open");
await settle(epage, before);
await ready(epage);
for (const tab of ["model", "grid", "yaml", "files"]) {
  await epage.click(`[data-tab="${tab}"]`);
  if (await epage.evaluate(() => document.querySelectorAll("img").length || window.pwned)) fail(`the ${tab} view runs or draws the link's HTML`);
}
await epage.click('[data-tab="grid"]');
if (!(await epage.locator("#view-grid").innerText()).includes("<img src=x onerror=window.pwned=1>")) fail("the grid does not show the link's text as text");

// And had an escape been missed, the policy stops what it would do: an
// inline handler does not run, and no other server can be reached
probing = true;
const stopped = await epage.evaluate(async () => {
  document.body.insertAdjacentHTML("beforeend", '<img id="probe" src="icon.svg?probe" onload="window.pwned=3" onerror="window.pwned=3">');
  await new Promise((r) => setTimeout(r, 500));
  document.querySelector("#probe").remove();
  const ran = window.pwned;
  let reached = true;
  try { await fetch("https://example.com/"); } catch (e) { reached = false; }
  await new Promise((r) => setTimeout(r, 500));
  const seen = window.blocked.splice(0);
  return { ran, reached, seen };
});
probing = false;
if (stopped.ran) fail("an inline handler ran: the Content Security Policy does not hold");
if (stopped.reached) fail("the page reached another server: the Content Security Policy does not hold");
if (!stopped.seen.some((b) => b.startsWith("script-src")) || !stopped.seen.some((b) => b.startsWith("connect-src"))) fail(`the policy did not report what it stopped: ${stopped.seen.join(", ")}`);

// Nothing a link carries reached the server, and the policy blocked nothing
for (const p of [page, mpage, epage]) {
  const blocked = await p.evaluate(() => window.blocked);
  for (const b of blocked) fail(`the Content Security Policy blocked ${b}`);
}
const secrets = [...Object.values(links), made.slice(base.length), "gzip=", "yaml=", "name="];
for (const r of requests) for (const s of secrets) if (r.includes(s.slice(0, 60))) fail(`a request to the server carries part of a link: ${r.slice(0, 160)}`);
if (requests.length < 10) fail(`the server saw only ${requests.length} requests`);

await browser.close();
server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.slice(0, 60).join("\n"));
  process.exit(1);
}
console.log(`Links: the mortgage, the bagel shop with its costs and the starter model open from a link with forge calculate's numbers; a reload keeps them, a link over your own files asks first, ${BROKEN.length} broken links say why, Files' link opens the same model elsewhere, a link's HTML is drawn as text, the policy stops an inline handler and a request elsewhere, and none of the server's ${requests.length} requests carried a model.`);
