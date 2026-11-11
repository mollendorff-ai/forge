// Forge Studio offline (service worker, sw.js), in a headless browser:
//
// - after the first load every file of the built site is on the device;
// - with the connection cut, the page opens and Forge calculates as it did
//   online (same state, no request fails);
// - a new release is a new cache, taken over only once Studio has been
//   closed, and the old cache is then deleted.
//
//   node studio/tests/offline.mjs <built Studio dir>

import { createServer } from "node:http";
import { readFileSync, existsSync, statSync, readdirSync, cpSync, mkdtempSync, appendFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, extname, resolve } from "node:path";
import { chromium } from "playwright";
import { writeServiceWorker } from "../build.mjs";

const v1 = resolve(process.argv[2]);
const v2 = mkdtempSync(join(tmpdir(), "studio-v2-"));
cpSync(v1, v2, { recursive: true });
appendFileSync(join(v2, "studio.css"), "\n/* a later release */\n");
writeServiceWorker(v2);

const files = (dir, prefix = "") => readdirSync(dir, { withFileTypes: true }).flatMap((e) =>
  e.isDirectory() ? files(join(dir, e.name), `${prefix}${e.name}/`) : e.name === "sw.js" || e.name === ".nojekyll" ? [] : [`${prefix}${e.name}`]);

let dir = v1;
const TYPES = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json", ".wasm": "application/wasm", ".webmanifest": "application/manifest+json", ".svg": "image/svg+xml", ".png": "image/png" };
const server = createServer((req, res) => {
  let file = join(dir, decodeURIComponent(new URL(req.url, "http://x").pathname));
  if (existsSync(file) && statSync(file).isDirectory()) file = join(file, "index.html");
  if (!file.startsWith(dir) || !existsSync(file)) { res.writeHead(404); res.end(); return; }
  res.writeHead(200, { "content-type": TYPES[extname(file)] || "application/octet-stream", "cache-control": "no-store" });
  res.end(readFileSync(file));
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const base = `http://127.0.0.1:${server.address().port}`;

const failures = [];
const fail = (m) => failures.push(m);
const browser = await chromium.launch();
const context = await browser.newContext({ viewport: { width: 390, height: 844 } });
const opened = async (url) => {
  const page = await context.newPage();
  const failed = [];
  page.on("pageerror", (e) => fail(e.message));
  page.on("requestfailed", (r) => failed.push(r.url()));
  try {
    await page.goto(url);
    await page.waitForFunction(() => document.body.dataset.state !== "loading", null, { timeout: 120000 });
  } catch (e) {
    fail(`${url} did not open: ${e.message.split("\n")[0]}`);
  }
  return { page, failed };
};
// waitForFunction does not wait for an async predicate, so poll
const waitCaches = async (page, count, what) => {
  for (let i = 0; i < 120; i++) {
    if ((await cacheNames(page)).length === count) return true;
    await new Promise((r) => setTimeout(r, 500));
  }
  fail(what);
  return false;
};
const cacheNames = (page) => page.evaluate(async () => (await caches.keys()).filter((n) => n.startsWith("forge-studio-")));

// Online, first time
const first = await opened(`${base}/`);
const state = await first.page.evaluate(() => document.body.dataset.state);
await first.page.evaluate(() => navigator.serviceWorker.ready);
await waitCaches(first.page, 1, "the first load did not keep the page for offline");
const names = await cacheNames(first.page);
if (names.length !== 1) fail(`expected one cache, found ${names.join(", ") || "none"}`);
else {
  const kept = await first.page.evaluate(async (n) => (await (await caches.open(n)).keys()).map((r) => new URL(r.url).pathname.replace(/^\//, "")), names[0]);
  const expected = ["", ...files(v1)];
  for (const f of expected) if (!kept.includes(f)) fail(`${f || "the page"} is not kept for offline`);
}
await first.page.close();

// Offline
await context.setOffline(true);
const off = await opened(`${base}/?example=mortgage`);
const offState = await off.page.evaluate(() => document.body?.dataset.state).catch(() => "no page");
if (offState !== state) fail(`offline the page is "${offState}", online it was "${state}"`);
const title = (await off.page.textContent("#model-title", { timeout: 2000 }).catch(() => "")) || "nothing";
if (!/mortgage/i.test(title)) fail(`offline the page shows "${title}", not the mortgage example`);
for (const u of off.failed) fail(`offline, a request failed: ${u}`);
await off.page.close();
await context.setOffline(false);

// A later release: a new cache next to the old one, which goes once Studio is closed
dir = v2;
const second = await opened(`${base}/`);
await waitCaches(second.page, 2, "a later release did not come to the device next to the first");
await second.page.close();
await new Promise((r) => setTimeout(r, 1000));
const third = await opened(`${base}/`);
await waitCaches(third.page, 1, "the first release's cache was not deleted once the later one took over");
const now = await cacheNames(third.page);
if (now[0] === names[0]) fail(`the later release did not replace the first one's cache (${names[0]} -> ${now.join(", ")})`);
await third.page.close();

await browser.close();
server.close();
if (failures.length) {
  console.error(`${failures.length} problems:\n` + failures.join("\n"));
  process.exit(1);
}
console.log("Offline: every file is kept on the device, the page opens and calculates with the connection cut, and a later release replaces the cache.");
