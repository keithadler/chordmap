// Runs the WebAssembly analysis off the main thread. Receives mono samples,
// posts back the analysis JSON. Nothing here touches the network.
import init, { analyze, chordSheet } from "./pkg/chordmap.js?v=dev";

// The wasm URL carries the build stamp too, so a new deploy is never served
// a stale binary from the browser or the CDN cache.
let ready = init({ module_or_path: new URL("./pkg/chordmap_bg.wasm?v=dev", import.meta.url) });

self.onmessage = async (e) => {
  const { id, samples, sampleRate, options } = e.data;
  try {
    await ready;
    const json = analyze(samples, sampleRate, JSON.stringify(options || {}));
    const sheet = chordSheet(json);
    self.postMessage({ id, ok: true, json, sheet });
  } catch (err) {
    self.postMessage({ id, ok: false, error: String(err && err.message ? err.message : err) });
  }
};
