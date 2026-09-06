// Caches the app shell so charts still work with no signal, backstage or
// on the road. The version is stamped by the deploy workflow; a new deploy
// replaces the cache on the next load.
const VERSION = "__VERSION__";
const CACHE = "chordmap-" + VERSION;
const SHELL = ["./", "./index.html", "./app.js?v=" + VERSION, "./worker.js?v=" + VERSION, "./chords-guitar.js?v=" + VERSION, "./export.js?v=" + VERSION, "./pkg/chordmap.js?v=" + VERSION, "./pkg/chordmap_bg.wasm?v=" + VERSION, "./manifest.webmanifest"];

self.addEventListener("install", (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(SHELL)).then(() => self.skipWaiting()));
});
self.addEventListener("activate", (e) => {
  e.waitUntil(caches.keys().then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)))).then(() => self.clients.claim()));
});
self.addEventListener("fetch", (e) => {
  if (e.request.method !== "GET" || new URL(e.request.url).origin !== location.origin) return;
  e.respondWith(caches.match(e.request, { ignoreSearch: false }).then((hit) => hit || fetch(e.request).then((res) => {
    if (res.ok) { const copy = res.clone(); caches.open(CACHE).then((c) => c.put(e.request, copy)); }
    return res;
  }).catch(() => caches.match("./index.html"))));
});
