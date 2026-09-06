// Runs the WebAssembly analysis off the main thread. Receives mono samples,
// posts back the analysis JSON. Nothing here touches the network.
import init, { analyze, chordSheet } from "./pkg/chordmap.js?v=dev";

let ready = init();

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
