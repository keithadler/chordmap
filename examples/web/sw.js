// Caches the app shell so charts still work with no signal, backstage or
// on the road. The version is stamped by the deploy workflow; a new deploy
// replaces the cache on the next load.
const VERSION = "__VERSION__";
const CACHE = "chordmap-" + VERSION;
const SHELL = ["./", "./index.html", "./app.js?v=" + VERSION, "./worker.js?v=" + VERSION, "./chords-guitar.js?v=" + VERSION, "./export.js?v=" + VERSION, "./library.js?v=" + VERSION, "./waveform.js?v=" + VERSION, "./visualizer.js?v=" + VERSION, "./stems.js?v=" + VERSION, "./mix.js?v=" + VERSION, "./lyrics.js?v=" + VERSION, "./icons/icon-192.png", "./icons/icon-512.png", "./pkg/chordmap.js?v=" + VERSION, "./pkg/chordmap_bg.wasm?v=" + VERSION, "./manifest.webmanifest"];

self.addEventListener("install", (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(SHELL)).then(() => self.skipWaiting()));
});
self.addEventListener("activate", (e) => {
  e.waitUntil(caches.keys().then((keys) => Promise.all(keys.filter((k) => k !== CACHE).map((k) => caches.delete(k)))).then(() => self.clients.claim()));
});
self.addEventListener("fetch", (e) => {
  const url = new URL(e.request.url);
  if (e.request.method !== "GET" || url.origin !== location.origin) return;
  // The neural runtime and models are big and cached by the page itself.
  if (url.pathname.includes("/vendor/") || url.pathname.includes("/local/")) return;
  // The page itself is fetched fresh whenever there is a signal, so a new
  // deploy shows up on the very next visit; the cache is only for offline.
  const isPage = e.request.mode === "navigate" || url.pathname.endsWith("/") || url.pathname.endsWith("/index.html");
  if (isPage) {
    e.respondWith(fetch(e.request).then((res) => {
      if (res.ok) { const copy = res.clone(); caches.open(CACHE).then((c) => c.put("./index.html", copy)); }
      return res;
    }).catch(() => caches.match("./index.html")));
    return;
  }
  e.respondWith(caches.match(e.request, { ignoreSearch: false }).then((hit) => hit || fetch(e.request).then((res) => {
    if (res.ok) { const copy = res.clone(); caches.open(CACHE).then((c) => c.put(e.request, copy)); }
    return res;
  })));
});
