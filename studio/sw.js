// Forge Studio offline: every file of the page is kept on the device the first
// time it loads, so the page opens, and Forge calculates, without a connection.
// build.mjs fills in the list of files and a name made from their content, so
// a new release is a new cache. It is not taken over while the page is open
// (no skipWaiting): the next time Studio is opened it is the new release,
// whole, never half of each.

const CACHE = "forge-studio-__BUILD__";
const FILES = __FILES__;

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(CACHE).then((cache) => cache.addAll(FILES)));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches.keys()
      .then((names) => Promise.all(names.filter((n) => n.startsWith("forge-studio-") && n !== CACHE).map((n) => caches.delete(n))))
      .then(() => self.clients.claim()),
  );
});

// The page is its own files: ?example=bagel is the same page as without it.
self.addEventListener("fetch", (event) => {
  if (event.request.method !== "GET") return;
  event.respondWith(
    caches.match(event.request, { cacheName: CACHE, ignoreSearch: true }).then((hit) => hit || fetch(event.request)),
  );
});
